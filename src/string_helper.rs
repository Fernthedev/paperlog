//! String helpers. Currently: finding the first non-ASCII byte, used by the
//! log chunker's fast path.
//!
//! Note on SIMD support: `bstr::ByteSlice::find_non_ascii_byte` is only
//! SIMD-accelerated on `x86_64`, where it uses SSE2 (part of the x86_64
//! baseline, so it needs no runtime detection). On every other target —
//! including aarch64, which is what the Quest runs — `bstr` falls back to a
//! portable word-at-a-time loop that checks 8 bytes per step.
//!
//! This module adds a NEON implementation for aarch64 (NEON is mandatory
//! there, so again no runtime detection) and defers to `bstr` everywhere
//! else. `core::simd` could cover every target with a single
//! implementation, but it is still nightly-only (`portable_simd`), and this
//! crate builds on stable.

/// Returns the index of the first byte `>= 0x80` in `bytes`, if any.
#[inline]
pub(crate) fn find_non_ascii_byte(bytes: &[u8]) -> Option<usize> {
    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little",
        not(miri)
    ))]
    {
        neon::find_non_ascii_byte(bytes)
    }

    #[cfg(not(all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little",
        not(miri)
    )))]
    {
        bstr::ByteSlice::find_non_ascii_byte(bytes)
    }
}

#[cfg(all(
    target_arch = "aarch64",
    target_feature = "neon",
    target_endian = "little",
    not(miri)
))]
mod neon {
    use core::arch::aarch64::{
        uint8x16_t, vget_lane_u64, vld1q_u8, vmaxvq_u8, vorrq_u8, vreinterpret_u64_u8,
        vreinterpretq_s8_u8, vreinterpretq_u16_u8, vreinterpretq_u8_s8, vshrn_n_u16, vshrq_n_s8,
    };

    /// Bytes per NEON register.
    const LANES: usize = 16;
    /// Bytes checked per iteration of the main loop (4 registers).
    const BLOCK: usize = 4 * LANES;

    pub(super) fn find_non_ascii_byte(bytes: &[u8]) -> Option<usize> {
        let len = bytes.len();
        // Below one register, vector setup isn't worth it.
        if len < LANES {
            return bytes.iter().position(|&b| b >= 0x80);
        }

        let ptr = bytes.as_ptr();
        let mut i = 0;

        // Main loop: OR four registers together and take one horizontal max,
        // so the common all-ASCII case costs a single reduction per 64 bytes.
        // On a hit, stop and let the 16-byte loop below find the exact byte.
        while i + BLOCK <= len {
            // SAFETY: `i + BLOCK <= len`, so all four 16-byte loads are in
            // bounds. `vld1q_u8` has no alignment requirement.
            let any_high = unsafe {
                let p = ptr.add(i);
                let v = vorrq_u8(
                    vorrq_u8(vld1q_u8(p), vld1q_u8(p.add(LANES))),
                    vorrq_u8(vld1q_u8(p.add(2 * LANES)), vld1q_u8(p.add(3 * LANES))),
                );
                vmaxvq_u8(v) >= 0x80
            };
            if any_high {
                break;
            }
            i += BLOCK;
        }

        while i + LANES <= len {
            // SAFETY: `i + LANES <= len`.
            if let Some(offset) = unsafe { first_high_byte(vld1q_u8(ptr.add(i))) } {
                return Some(i + offset);
            }
            i += LANES;
        }

        if i == len {
            return None;
        }

        // Fewer than 16 bytes remain: re-check the *last* 16 bytes instead
        // (`len >= LANES` here). The overlap only covers bytes before `i`,
        // which are already known to be ASCII, so any hit is at or after `i`.
        let start = len - LANES;
        // SAFETY: `start + LANES == len`.
        unsafe { first_high_byte(vld1q_u8(ptr.add(start))) }.map(|offset| start + offset)
    }

    /// Index of the first byte `>= 0x80` in `v`, if any.
    #[inline(always)]
    unsafe fn first_high_byte(v: uint8x16_t) -> Option<usize> {
        // NEON has no x86-style `movemask`, so build one from `shrn`:
        // 1. Arithmetic shift right by 7 turns each byte into 0x00 (ASCII)
        //    or 0xFF (high bit set).
        // 2. Viewing that as eight u16 lanes, shift each right by 4 and
        //    narrow to u8. Byte `2k` of the input lands in the low nibble of
        //    output byte `k` and byte `2k + 1` in its high nibble, so the
        //    64-bit result holds one nibble per input byte, in order
        //    (little-endian).
        // SAFETY: plain register operations, no memory access.
        let nibbles = unsafe {
            let mask = vreinterpretq_u8_s8(vshrq_n_s8::<7>(vreinterpretq_s8_u8(v)));
            vget_lane_u64::<0>(vreinterpret_u64_u8(vshrn_n_u16::<4>(vreinterpretq_u16_u8(
                mask,
            ))))
        };

        (nibbles != 0).then(|| (nibbles.trailing_zeros() / 4) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::find_non_ascii_byte;

    fn reference(bytes: &[u8]) -> Option<usize> {
        bytes.iter().position(|&b| b >= 0x80)
    }

    /// Covers every length up to a few blocks (short scalar path, 64-byte
    /// loop, 16-byte loop, overlapping tail) with a high byte at every
    /// position, and with no high byte at all. On aarch64 this exercises
    /// the NEON path; elsewhere it checks the `bstr` fallback.
    #[test]
    fn matches_reference_for_every_length_and_position() {
        for len in 0..=200 {
            let ascii = vec![b'a'; len];
            assert_eq!(find_non_ascii_byte(&ascii), None, "len {len}");

            for pos in 0..len {
                for high in [0x80u8, 0xC3, 0xFF] {
                    let mut bytes = ascii.clone();
                    bytes[pos] = high;
                    // A second high byte later must not change the answer.
                    if pos + 7 < len {
                        bytes[pos + 7] = 0xE2;
                    }
                    assert_eq!(
                        find_non_ascii_byte(&bytes),
                        reference(&bytes),
                        "len {len}, pos {pos}, byte {high:#x}"
                    );
                }
            }
        }
    }

    #[test]
    fn unaligned_starts() {
        let mut bytes = vec![b'x'; 300];
        bytes[250] = 0xF0;
        for start in 0..32 {
            let slice = &bytes[start..];
            assert_eq!(
                find_non_ascii_byte(slice),
                reference(slice),
                "start {start}"
            );
        }
    }
}

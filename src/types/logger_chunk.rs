use std::fmt;

use bstr::ByteSlice;

use crate::types::{time::LocalTime, LogData, PaperStr};

/// Line ending written after each line in log files: the platform's native
/// one.
pub const LINE_END: &[u8] = if cfg!(windows) { b"\r\n" } else { b"\n" };

#[derive(Debug, Clone, Copy)]
pub struct LogDataChunk<'a> {
    pub original: &'a LogData,
    pub message: &'a PaperStr,
}

impl<'a> LogDataChunk<'a> {
    /// Copies this chunk into a standalone [`LogData`] whose message is
    /// just this chunk's slice of the original message.
    pub fn to_log_data(&self) -> LogData {
        LogData {
            message: self.message.to_owned(),
            ..self.original.clone()
        }
    }

    /// Borrows the tag as bytes, falling back to the default tag. See
    /// [`LogData::tag_or_default`].
    pub fn tag(&self) -> &'a PaperStr {
        self.original.tag_or_default()
    }

    fn time(&self) -> LocalTime {
        LocalTime(self.original.timestamp)
    }

    /// Formats the chunk the same way its [`Display`](fmt::Display) impl
    /// does. Prefer writing `chunk` straight into a writer/formatter when
    /// you have one; this allocates a `String`.
    pub fn format(&self) -> String {
        self.to_string()
    }

    /// Writes the full line (short level, tag), ended with the platform's
    /// line ending ([`LINE_END`]).
    pub fn write_to_io(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(
            writer,
            "{level} {time} [{tag}] [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.original.level,
            time = self.time(),
            tag = self.tag(),
            message = self.message,
            line = self.original.line,
            column = self.original.column,
            file = self.original.file,
            function_name = self.original.function_name_or_default(),
        )?;
        writer.write_all(LINE_END)
    }

    /// Writes the compact line (no tag — used for per-context files, where
    /// the tag is implied by the file), ended with [`LINE_END`].
    pub fn write_compact_to_io(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(
            writer,
            "{level} {time} [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.original.level.short(),
            time = self.time(),
            message = self.message,
            line = self.original.line,
            column = self.original.column,
            file = self.original.file,
            function_name = self.original.function_name_or_default(),
        )?;
        writer.write_all(LINE_END)
    }
}

/// The human-readable form (long level name), used by stdout and tracing.
/// No trailing newline.
impl fmt::Display for LogDataChunk<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{level} {time} [{tag}] [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.original.level.short(),
            time = self.time(),
            tag = self.tag(),
            message = self.message,
            line = self.original.line,
            column = self.original.column,
            file = self.original.file,
            function_name = self.original.function_name_or_default(),
        )
    }
}

/// Byte offset at which `s` should be split so the head holds exactly
/// `max_chars` characters (or all of `s`, if it has fewer).
///
/// UTF-8 aware: the split point is always a character boundary, so a
/// multi-byte character is never cut in half. Invalid bytes are counted the
/// same way `bstr`'s `char_indices` counts them (each maximal invalid
/// subsequence is one replacement character), so arbitrary non-UTF-8 input
/// still chunks consistently and every byte is preserved.
fn split_offset(s: &PaperStr, max_chars: usize) -> usize {
    // A character is at least one byte, so a string of at most `max_chars`
    // bytes has at most `max_chars` characters: no decoding needed. This is
    // the common case for log lines.
    if s.len() <= max_chars {
        return s.len();
    }

    // Every byte before the first non-ASCII byte is exactly one character.
    // SIMD-accelerated: NEON on aarch64, SSE2 (via `bstr`) on x86_64; see
    // `crate::string_helper`.
    let ascii_prefix =
        crate::string_helper::find_non_ascii_byte(&s[..max_chars]).unwrap_or(max_chars);
    if ascii_prefix == max_chars {
        // The first `max_chars` characters are all single-byte, so the next
        // character (whatever it is) starts right at `max_chars`.
        return max_chars;
    }

    // `ascii_prefix` is a character boundary (everything before it is
    // ASCII), so decoding can resume there with the remaining budget
    // instead of re-decoding the prefix.
    let tail = s[ascii_prefix..].as_bstr();
    tail.char_indices()
        .nth(max_chars - ascii_prefix)
        .map_or(s.len(), |(start, _, _)| ascii_prefix + start)
}

/// Helper to chunk a BStr by character count instead of raw byte count.
///
/// `max_chars == 0` means "no limit" (one chunk per line); otherwise the
/// iterator could never make progress.
fn chunk_by_chars(s: &PaperStr, max_chars: usize) -> impl Iterator<Item = &PaperStr> {
    let max_chars = if max_chars == 0 {
        usize::MAX
    } else {
        max_chars
    };
    let mut remaining = s;

    std::iter::from_fn(move || {
        if remaining.is_empty() {
            return None;
        }

        let (chunk, tail) = remaining.split_at(split_offset(remaining, max_chars));
        remaining = tail.as_bstr();
        Some(chunk.as_bstr())
    })
}

/// Split log message by line endings and then split each line into character-aligned chunks
pub fn split_str_into_chunks(
    queue: &[LogData],
    max_chars: usize,
) -> impl Iterator<Item = LogDataChunk<'_>> + '_ {
    queue.iter().flat_map(move |log| {
        log.message
            .lines() // Splits by line endings (\n or \r\n) safely
            .flat_map(move |line| {
                chunk_by_chars(line.as_bstr(), max_chars).map(move |chunk| LogDataChunk {
                    original: log,
                    message: chunk,
                })
            })
    })
}

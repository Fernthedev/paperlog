# AGENTS.md

## Project overview
This repository is the Paper logging library for Rust-based Quest/Android mods. It is a Cargo workspace with the main crate (`paper2`) at the repo root plus helper crates: `paper2_log` (a `log`-crate facade that forwards into paper2's C API), `paper2_tracing`, `paper2_ffi` (bindgen bindings generated from the cbindgen-generated `shared/bindings.h`, for Rust mods that dynamically link against paper2), `questloader`, and `scotland2`. The `shared/` directory holds the generated C header plus hand-written C++ wrapper headers (`logger.hpp`, etc.) that native (C++/QuestLoader) mods include directly.

### Module layout (`src/`)
- `lib.rs` — crate root; owns the global `LOGGER: OnceLock<ThreadSafeLoggerThread>` singleton and `get_logger()`/`init_logger()`.
- `logger.rs` — `LoggerThreadCtx` (the core logger: queueing, the dedicated background logging thread, context files, sinks) plus a private `sinks` submodule (`logger/sinks/{file,logcat,sink,stdout,tracing}_logger.rs`) — one backend per file, each feature-gated.
- `types/` — `LogData` (a single log entry), `logger_chunk.rs` (`LogDataChunk` + the line/char chunking logic that respects `max_string_len`), `log_level.rs`, `config.rs` (`LoggerConfig`), and the `PaperStr`/`PaperString` type aliases.
- `ffi.rs` (+ `ffi/c_str_helper.rs`) — the `#[no_mangle] extern "C"` surface. Feature-gated behind `ffi` (a default feature).
- `vec_pool.rs`, `semaphore_lite.rs` — small internal utilities, not part of the public API.

## Working rules for AI agents
- Prefer small, targeted changes that preserve existing behavior.
- Keep logging semantics deterministic, thread-safe, and compatible with the existing feature-gated build.
- Do not break FFI or Android-specific paths unless the task explicitly requires it.
- When changing public APIs, re-exported symbols, or crate boundaries, update the relevant crates and docs.
- Preserve compatibility with the existing C++/shared integration under `shared/`.

### Byte-string / non-UTF-8 invariant
`LogData`'s `tag`/`message`/`file`/`function_name` fields, and the FFI's `StringRef`, are `bstr` (`PaperStr`/`PaperString`) or raw `(ptr, len)` byte pairs — **not** `String`/`&str`. This is deliberate: the primary caller is native C/C++ code passing arbitrary byte buffers, and paper2 must never reject, panic on, or lossily rewrite bytes the caller wrote just because they aren't valid UTF-8. Concretely:
- Don't reintroduce `String`/`&str` for these fields, and don't add `.to_str()`/`to_string_lossy()`-style validation on the write path — use `bstr`'s `ByteSlice`/`ByteVec` helpers (`to_path_lossy`, `from_path_lossy`, `as_bstr`, etc.), which are zero-copy on unix.
- `BString` (`PaperString`) derefs to `Vec<u8>`, **not** `BStr` — `.as_deref()` on an `Option<PaperString>` gives `Option<&Vec<u8>>`, not `Option<&PaperStr>`. Use `.as_ref().map(|s| s.as_bstr())` instead (see `LogData::tag_or_default()`).
- Display formatting of `PaperStr`/`PaperString` (for the text log files, `tracing`/`stdout` backends) *is* lossy for invalid UTF-8 — that's expected and fine, since that's human-readable output, not the byte-fidelity path. The FFI sink callback (`LogDataC`/`StringRef`) and file-routing-by-tag (`context_map`) must stay byte-exact.
- The logging thread reads whole batches of `LogData` then chunks them into `LogDataChunk`s (borrowed metadata + a message slice) to avoid allocating a new `LogData` per output line/backend — keep new backends taking `LogDataChunk<'_>` rather than `&LogData`, consistent with the existing ones in `logger/sinks/`.

### Testing an Android-specific feature (`logcat`, `android-api-30`)
Code under `#[cfg(feature = "logcat")]` needs `ndk-sys`, which is only a dependency for `target_os = "android"` — it **cannot** be `cargo check`'d on a Linux/Mac dev host. Don't skip verifying it: use `cargo-ndk` (already installed in this environment) against the real target instead, e.g.:
```bash
export ANDROID_NDK_HOME=$(cat ndkpath.txt)
cargo ndk -t arm64-v8a clippy -p paper2 --no-default-features --features ffi,file,logcat,android-api-30
```

## Build and test
Use the repo root as the working directory.

```bash
cargo test --workspace          # unit tests (src/tests/) + integration tests (tests/)
cargo test --workspace --lib    # unit tests only
cargo bench --workspace --no-run
```

Tests are split by what they can reach:
- `tests/*.rs` — integration tests, one file per area (`log.rs`, `logger_impl.rs`, `logger_init.rs`, `logger_chunk.rs`, `ffi.rs`, `non_utf8.rs`). These only see `paper2`'s **public** API (as `paper2::...`) and its `[dev-dependencies]` — not its ordinary `[dependencies]` (e.g. `bstr` isn't reachable directly; use inherent slice methods like `.to_vec()` instead of `bstr::ByteSlice` trait methods).
- `src/tests/*.rs` (registered in `src/tests/mod.rs`) — unit tests, reserved for cases that need crate-private items (e.g. `vec_pool.rs` uses `pub(crate) fn pool_len`, `semaphore_lite.rs` tests a private module). Prefer integration tests for anything expressible through the public API; move a test out of `src/tests/` if a later refactor makes it public-API-only.
- `tests/ffi.rs`'s `paper2_init_logger_ffi` populates a process-global `OnceLock` that only initializes once per process — keep any test that depends on that shared global state (init/get_inited ordering, etc.) in that one test function rather than spreading it across tests that could race for who "wins" the init. (Each `tests/*.rs` file is its own process, so this doesn't affect the other integration test files.)
- `benches/`: `logger_bench.rs` (queue/flush throughput), `chunking_bench.rs` (the line/char splitting logic), `ffi_bench.rs` (raw C-API call overhead), `lock_compare.rs` (`parking_lot` vs `std` microbenchmarks, self-contained).

## Toolchain
- The workspace uses nightly Rust via `rust-toolchain.toml`.
- Avoid introducing dependencies that would require major ecosystem changes without discussion.

## Notes
- Feature flags: `global_log`, `context_log`, `ffi`, `backtrace`, `file`, `sinks` (all default); `logcat`, `stdout`, `tracing`, `android-api-30` (opt-in, Android/NDK-only or mutually exclusive with `tracing` — see the `compile_error!` guards in `logger/sinks/`).
- If you change logging behavior, consider whether existing tests or README examples need updates.

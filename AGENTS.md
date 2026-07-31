# AGENTS.md

## Project overview
This repository is the Paper logging library for Rust-based Quest/Android mods. It is a Cargo workspace with the main crate at the repo root plus helper crates such as paper2_log, paper2_tracing, paper2_ffi, questloader, and scotland2.

## Working rules for AI agents
- Prefer small, targeted changes that preserve existing behavior.
- Keep logging semantics deterministic, thread-safe, and compatible with the existing feature-gated build.
- Do not break FFI or Android-specific paths unless the task explicitly requires it.
- When changing public APIs, re-exported symbols, or crate boundaries, update the relevant crates and docs.
- Preserve compatibility with the existing C++/shared integration under shared/.

## Build and test
Use the repo root as the working directory.

```bash
cargo test --workspace --lib
cargo test --workspace
cargo bench --workspace --no-run
```

## Toolchain
- The workspace uses nightly Rust via rust-toolchain.toml.
- Avoid introducing dependencies that would require major ecosystem changes without discussion.

## Notes
- This project has feature flags such as global_log, context_log, ffi, backtrace, file, sinks, logcat, and stdout.
- If you change logging behavior, consider whether existing tests or README examples need updates.

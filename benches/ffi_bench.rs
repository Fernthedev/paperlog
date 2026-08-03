use std::ffi::CString;
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use paper2::ffi::{paper2_get_inited, paper2_init_logger_ffi, paper2_queue_log_ffi};
use paper2::types::log_level::LogLevel;

// `paper2_init_logger_ffi` only actually initializes the process-global
// logger once; later calls (including from other benches in this binary)
// are safe no-ops, so it's fine to call this from every bench function.
fn ensure_inited() {
    // See `benches/logger_bench.rs`: keep bench log files off whatever
    // filesystem the checkout happens to be on (may be slow/inconsistent,
    // e.g. FUSE-backed) and use the OS temp dir instead.
    let log_path = std::env::temp_dir()
        .join("paper2_bench")
        .join("ffi_bench.log");
    let log_path = CString::new(log_path.to_str().unwrap()).unwrap();
    unsafe {
        paper2_init_logger_ffi(std::ptr::null(), log_path.as_ptr());
    }
}

// Measures the cost of `paper2_get_inited`, called on every line logged
// through the `log`-crate facade. Should be a single lock-free `OnceLock`
// load — see the comment on `get_logger()` for why.
fn bench_paper2_get_inited(c: &mut Criterion) {
    ensure_inited();

    c.bench_function("paper2_get_inited", |b| {
        b.iter(|| black_box(unsafe { paper2_get_inited() }))
    });
}

// Measures the full cost of a single `paper2_queue_log_ffi` call: the
// `get_logger()` lookup, the two required `CString` byte copies (message
// and file), and the channel send. This is the primary hot path exercised
// by every C-side log call.
fn bench_paper2_queue_log_ffi(c: &mut Criterion) {
    ensure_inited();

    let message = CString::new("benchmark message via the C API").unwrap();
    let file = CString::new(file!()).unwrap();

    c.bench_function("paper2_queue_log_ffi", |b| {
        b.iter(|| {
            let ok = unsafe {
                paper2_queue_log_ffi(
                    LogLevel::Debug,
                    std::ptr::null(),
                    message.as_ptr(),
                    file.as_ptr(),
                    line!() as i32,
                    column!() as i32,
                    std::ptr::null(),
                )
            };
            black_box(ok);
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(50);
    targets = bench_paper2_get_inited, bench_paper2_queue_log_ffi
}
criterion_main!(benches);

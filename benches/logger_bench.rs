use criterion::{criterion_group, criterion_main, Criterion};
use paper2::{
    logger::LoggerThreadCtx,
    types::{config::LoggerConfig, log_level::LogLevel, LogData},
};
use std::time::Duration;

// Benches write real log files, so their directory matters: the repo
// checkout may sit on a slow/inconsistent mount (e.g. a FUSE-backed drive),
// and file I/O timing would then dominate whatever this bench is actually
// meant to measure. Use the OS temp dir instead, same reasoning as
// `tests/non_utf8.rs`.
fn bench_log_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("paper2_bench")
}

fn bench_config() -> LoggerConfig {
    LoggerConfig {
        context_log_path: bench_log_dir(),
        ..LoggerConfig::default()
    }
}

fn bench_queue_log(c: &mut Criterion) {
    let config = bench_config();
    let log_path = bench_log_dir().join("bench.log");
    let logger = LoggerThreadCtx::new(config, log_path)
        .unwrap()
        .init(true)
        .unwrap();

    let log_data = LogData {
        level: LogLevel::Info,
        message: "Benchmark log message".into(),
        ..Default::default()
    };

    c.bench_function("queue_log", |b| {
        b.iter(|| {
            logger.read().queue_log(log_data.clone());
        })
    });

    // `queue_log` only sends; it doesn't wait on the logging thread at all,
    // so criterion's measurement loop can enqueue far more entries than the
    // logging thread drains in the same window. A single flush wait only
    // blocks for the *next* flush pulse, not for the backlog to actually
    // empty — so without this loop, the leftover backlog keeps being
    // written in the background while the next bench function measures,
    // contaminating its timings.
    while logger.read().queue_len() > 0 {
        logger.read().wait_for_flush_timeout(Duration::from_secs(5));
    }
}

fn bench_log_100_and_flush(c: &mut Criterion) {
    let config = bench_config();
    let log_path = bench_log_dir().join("bench_100.log");
    let logger = LoggerThreadCtx::new(config, log_path)
        .unwrap()
        .init(true)
        .unwrap();

    let log_data = LogData {
        level: LogLevel::Info,
        message: "Benchmark log message".into(),
        ..Default::default()
    };

    c.bench_function("log_100_and_flush", |b| {
        b.iter(|| {
            for _ in 0..100 {
                logger.read().queue_log(log_data.clone());
            }
            logger.read().wait_for_flush_timeout(Duration::from_secs(5));
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(10);
    targets = bench_queue_log, bench_log_100_and_flush
}
criterion_main!(benches);

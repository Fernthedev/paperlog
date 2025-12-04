use criterion::{criterion_group, criterion_main, Criterion};
use paper2::{
    log_level::LogLevel,
    logger::{logger_thread_ctx::LoggerThreadCtx, LogData, LoggerConfig},
};
use std::{path::PathBuf, time::Duration};

fn bench_queue_log(c: &mut Criterion) {
    let config = LoggerConfig::default();
    let log_path = PathBuf::from("logs/bench.log");
    let logger = LoggerThreadCtx::new(config, log_path)
        .unwrap()
        .init(true)
        .unwrap();

    let log_data = LogData {
        level: LogLevel::Info,
        message: "Benchmark log message".to_string(),
        ..Default::default()
    };

    c.bench_function("queue_log", |b| {
        b.iter(|| {
            logger.read().queue_log(log_data.clone());
        })
    });

    logger.read().wait_for_flush_timeout(Duration::from_secs(5));
}

fn bench_log_100_and_flush(c: &mut Criterion) {
    let config = LoggerConfig::default();
    let log_path = PathBuf::from("logs/bench_100.log");
    let logger = LoggerThreadCtx::new(config, log_path)
        .unwrap()
        .init(true)
        .unwrap();

    let log_data = LogData {
        level: LogLevel::Info,
        message: "Benchmark log message".to_string(),
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

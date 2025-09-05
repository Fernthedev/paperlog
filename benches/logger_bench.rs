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
            logger.read().unwrap().queue_log(log_data.clone());
        })
    });

    logger.read().unwrap().wait_for_flush_timeout(Duration::from_secs(5));
}


criterion_group!{
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(10 * 1000); // 10 thousand logs
    targets = bench_queue_log
}
criterion_main!(benches);

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use paper2::types::log_level::LogLevel;
use paper2::types::logger_chunk::split_str_into_chunks;
use paper2::types::LogData;

fn make_log(message: String) -> LogData {
    LogData {
        level: LogLevel::Info,
        message: message.into(),
        ..Default::default()
    }
}

fn bench_chunk_short_message(c: &mut Criterion) {
    let queue = vec![make_log("a short log line".to_string())];

    c.bench_function("chunk_short_message", |b| {
        b.iter(|| {
            let chunks: Vec<_> = split_str_into_chunks(black_box(&queue), 1024).collect();
            black_box(chunks);
        })
    });
}

fn bench_chunk_long_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_long_message");

    for len in [256usize, 4096, 65536] {
        let queue = vec![make_log("x".repeat(len))];

        group.bench_with_input(BenchmarkId::from_parameter(len), &queue, |b, queue| {
            b.iter(|| {
                let chunks: Vec<_> = split_str_into_chunks(black_box(queue), 1024).collect();
                black_box(chunks);
            })
        });
    }

    group.finish();
}

fn bench_chunk_multiline_message(c: &mut Criterion) {
    let message = (0..200)
        .map(|i| format!("line number {i} of a longer multi-line log message"))
        .collect::<Vec<_>>()
        .join("\n");
    let queue = vec![make_log(message)];

    c.bench_function("chunk_multiline_message", |b| {
        b.iter(|| {
            let chunks: Vec<_> = split_str_into_chunks(black_box(&queue), 1024).collect();
            black_box(chunks);
        })
    });
}

fn bench_chunk_batch_of_short_messages(c: &mut Criterion) {
    // Mirrors the real hot path: `log_thread` drains a batch of queued log
    // entries and chunks them all together in one pass.
    let queue: Vec<_> = (0..100)
        .map(|i| make_log(format!("log entry number {i}")))
        .collect();

    c.bench_function("chunk_batch_of_100_short_messages", |b| {
        b.iter(|| {
            let chunks: Vec<_> = split_str_into_chunks(black_box(&queue), 1024).collect();
            black_box(chunks);
        })
    });
}

criterion_group!(
    benches,
    bench_chunk_short_message,
    bench_chunk_long_message,
    bench_chunk_multiline_message,
    bench_chunk_batch_of_short_messages
);
criterion_main!(benches);

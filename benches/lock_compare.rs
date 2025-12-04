use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use parking_lot::{Mutex as ParkingMutex, RwLock as ParkingRwLock};
use std::sync::{Mutex as StdMutex, RwLock as StdRwLock};

// Microbenchmarks that compare uncontended lock/unlock and read-lock heavy RwLock
// behavior between parking_lot and std. These are small, fast benches intended
// to reproduce relative regressions.

fn bench_mutex_uncontended(c: &mut Criterion) {
    let mut group = c.benchmark_group("mutex_uncontended");

    let m_std = StdMutex::new(());
    group.bench_function(BenchmarkId::new("std", "lock"), |b| {
        b.iter(|| {
            for _ in 0..1_000 {
                let _g = m_std.lock().unwrap();
            }
        })
    });

    let m_parking = ParkingMutex::new(());
    group.bench_function(BenchmarkId::new("parking", "lock"), |b| {
        b.iter(|| {
            for _ in 0..1_000 {
                let _g = m_parking.lock();
            }
        })
    });

    group.finish();
}

fn bench_rwlock_read_uncontended(c: &mut Criterion) {
    let mut group = c.benchmark_group("rwlock_read_uncontended");

    let r_std = StdRwLock::new(0usize);
    group.bench_function(BenchmarkId::new("std", "read"), |b| {
        b.iter(|| {
            for _ in 0..1_000 {
                let _g = r_std.read().unwrap();
            }
        })
    });

    let r_parking = ParkingRwLock::new(0usize);
    group.bench_function(BenchmarkId::new("parking", "read"), |b| {
        b.iter(|| {
            for _ in 0..1_000 {
                let _g = r_parking.read();
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mutex_uncontended,
    bench_rwlock_read_uncontended
);
criterion_main!(benches);

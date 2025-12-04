use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

use crate::semaphore_lite::SemaphoreLite;

#[test]
fn test_signal_wakes_waiter() {
    let sem = Arc::new(SemaphoreLite::new());
    let flag = Arc::new(AtomicBool::new(false));

    let s = sem.clone();
    let f = flag.clone();

    let handle = thread::spawn(move || {
        s.wait();
        f.store(true, Ordering::SeqCst);
    });

    // give the thread a moment to block on wait
    thread::sleep(Duration::from_millis(20));
    sem.signal();

    handle.join().expect("thread panicked");
    assert!(flag.load(Ordering::SeqCst));
}

#[test]
fn test_wait_timeout_returns() {
    let sem = SemaphoreLite::new();

    let start = Instant::now();
    sem.wait_timeout(Duration::from_millis(50));
    let elapsed = start.elapsed();

    // Ensure we returned after the timeout (allow some leeway)
    assert!(elapsed >= Duration::from_millis(45));
}

#[test]
fn test_signal_before_wait() {
    let sem = SemaphoreLite::new();

    sem.signal();
    let start = Instant::now();
    sem.wait();
    let elapsed = start.elapsed();

    // If signal happened before wait, wait should return immediately (very small)
    assert!(elapsed < Duration::from_millis(10));
}

#[test]
fn test_notify_all_wakes_one_only() {
    // Because SemaphoreLite stores a single boolean flag, notify_all will wake all
    // waiters but only the first to acquire the mutex will see the flag and
    // proceed; subsequent waiters will loop and block again. This test verifies
    // that behavior: with one signal and two waiters, only one progresses.

    let sem = Arc::new(SemaphoreLite::new());
    let flag1 = Arc::new(AtomicBool::new(false));
    let flag2 = Arc::new(AtomicBool::new(false));

    let s1 = sem.clone();
    let f1 = flag1.clone();
    let h1 = thread::spawn(move || {
        s1.wait();
        f1.store(true, Ordering::SeqCst);
    });

    let s2 = sem.clone();
    let f2 = flag2.clone();
    let h2 = thread::spawn(move || {
        // second thread waits a bit to increase the chance of contention
        s2.wait();
        f2.store(true, Ordering::SeqCst);
    });

    // allow both threads to block
    thread::sleep(Duration::from_millis(20));
    sem.signal();

    // give threads a moment to run
    thread::sleep(Duration::from_millis(50));

    let progressed = flag1.load(Ordering::SeqCst) as u8 + flag2.load(Ordering::SeqCst) as u8;

    // Only one should have progressed
    assert_eq!(
        progressed, 1,
        "expected exactly one waiter to progress, got {progressed}"
    );

    // Cleanup: signal again so the other thread can finish and join
    sem.signal();
    h1.join().ok();
    h2.join().ok();
}

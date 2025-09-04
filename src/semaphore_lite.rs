use std::{
    sync::{Condvar, Mutex}, time::Duration
};

/// A lightweight semaphore implementation using Mutex and Condvar.
/// 
/// This is not a full-featured semaphore, but provides basic signaling capabilities.
/// 
/// This type is used for simple thread signaling.
#[derive(Debug, Default)]
pub struct SemaphoreLite {
    mutex: Mutex<bool>,
    cond_var: Condvar,
}

impl SemaphoreLite {
    pub fn new() -> Self {
        SemaphoreLite {
            mutex: Mutex::new(false),
            cond_var: Condvar::new(),
        }
    }

    /// Signals one waiting thread.
    pub fn signal(&self) {
        let mut guard = self.mutex.lock().unwrap();
        *guard = true;
        self.cond_var.notify_all();
    }

    /// Waits until signaled.
    pub fn wait(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !*guard {
            guard = self.cond_var.wait(guard).unwrap();
        }
        *guard = false;
    }

    /// Waits until signaled or timeout occurs.
    pub fn wait_timeout(&self, duration: Duration) {
        let mut guard = self.mutex.lock().unwrap();
        while !*guard {
            let (guard_result, result) = self.cond_var.wait_timeout(guard, duration).unwrap();
            guard = guard_result;

            if result.timed_out() {
                return;
            }
        }
        *guard = false;
    }
}

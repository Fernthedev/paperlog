use std::{
    sync::{Condvar, Mutex},
    time::Duration,
};

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

    pub fn signal(&self) {
        let mut guard = self.mutex.lock().unwrap();
        *guard = true;
        self.cond_var.notify_all();
    }

    pub fn wait(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !*guard {
            guard = self.cond_var.wait(guard).unwrap();
        }
        *guard = false;
    }

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

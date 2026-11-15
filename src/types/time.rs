//! Cheap timestamp formatting for log lines.
//!
//! Logs only carry a raw [`SystemTime`] (a vDSO clock read, no timezone
//! work on the producer). Converting to local time and formatting it is
//! done lazily, when a line is written, and cached per thread for the
//! current second: nearly every line in a batch shares the same second, so
//! the timezone lookup and `strftime`-style formatting run about once per
//! second instead of once (or twice, with a context file) per line.

use std::{
    cell::RefCell,
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Local};

const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

struct Cache {
    /// Whole seconds since the Unix epoch that `formatted` was built for.
    second: u64,
    formatted: String,
}

thread_local! {
    static CACHE: RefCell<Cache> = const {
        RefCell::new(Cache { second: u64::MAX, formatted: String::new() })
    };
}

/// Displays a [`SystemTime`] as local `YYYY-MM-DD HH:MM:SS`.
#[derive(Clone, Copy)]
pub(crate) struct LocalTime(pub SystemTime);

impl LocalTime {
    fn write_uncached(&self, f: &mut impl fmt::Write) -> fmt::Result {
        let local: DateTime<Local> = self.0.into();
        write!(f, "{}", local.format(TIME_FORMAT))
    }
}

impl fmt::Display for LocalTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Pre-1970 timestamps don't have a `u64` key; just format them.
        let Ok(since_epoch) = self.0.duration_since(UNIX_EPOCH) else {
            return self.write_uncached(f);
        };
        let second = since_epoch.as_secs();

        // `try_with`/`try_borrow_mut`: fall back to uncached formatting
        // during thread-local teardown or (theoretically) re-entrancy,
        // rather than panicking inside a logger.
        CACHE
            .try_with(|cache| {
                let Ok(mut cache) = cache.try_borrow_mut() else {
                    return self.write_uncached(f);
                };
                if cache.second != second {
                    cache.formatted.clear();
                    self.write_uncached(&mut cache.formatted)?;
                    cache.second = second;
                }
                f.write_str(&cache.formatted)
            })
            .unwrap_or_else(|_| self.write_uncached(f))
    }
}

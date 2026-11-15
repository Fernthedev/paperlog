use std::{
    backtrace::Backtrace,
    panic::PanicHookInfo,
    sync::Arc,
    thread,
    time::{Duration, SystemTime},
};

#[cfg(feature = "file")]
use crate::types::file_logger_map::FileLoggers;
#[cfg(feature = "file")]
use crate::LoggerError;
use crate::{
    semaphore_lite::SemaphoreLite,
    types::config::LoggerConfig,
    types::log_level::LogLevel,
    types::{
        logger_chunk::{self, LogDataChunk},
        LogData, PaperStr,
    },
    Result,
};
use bstr::ByteSlice;
use crossbeam_channel::{self as channel, Receiver, Sender};
#[cfg(feature = "file")]
use parking_lot::Mutex;
use parking_lot::RwLock;

mod error;
pub mod sinks;

#[cfg(feature = "file")]
use error::report_internal;
use error::{ErrorReporter, ErrorSource};

/// A log sink: called with every chunk the logging thread writes.
///
/// Implemented for every matching closure, so `add_sink(|chunk| ...)` works
/// directly. (A blanket-implemented trait instead of a nightly
/// `trait_alias`, so the crate builds on stable.)
pub trait LogCallback: Fn(LogDataChunk<'_>) -> Result<()> + Send + Sync {}

impl<F> LogCallback for F where F: Fn(LogDataChunk<'_>) -> Result<()> + Send + Sync {}

// Helper macro to reduce repetition when constructing `LogData` and calling `do_log`.
// The macro performs `format!` internally — pass format-style arguments directly.
// Usage: log_data_to!(&logger, LogLevel::Error, Some("tag".into()), "msg: {}", val);
macro_rules! log_data_to {
    ($logger:expr, $level:expr, $tag:expr, $($arg:tt)+) => {{
        let message = format!($($arg)+).into();
        let _ = do_log(
            &LogData {
                level: $level,
                tag: $tag,
                message,
                timestamp: SystemTime::now(),
                file: file!().into(),
                line: line!(),
                column: column!(),
                function_name: None,
            },
            $logger,
        );
    }};
}

pub type ThreadSafeLoggerThread = Logger;

/// State shared between the [`Logger`] (producer/public API) and
/// [`LoggerThread`] (consumer, runs on the dedicated logging thread).
///
/// Grouping these behind a single `Arc` — rather than each field getting its
/// own `Arc` cloned into both structs — means cloning a `Logger` (done on
/// every FFI call site that grabs the global logger) bumps one refcount
/// instead of several.
///
/// Locking is kept off the hot path (`queue_log`/`queue_logs`, which only
/// touch the lock-free channel) by design:
/// - `config` is immutable after construction, so it needs no lock at all.
/// - `flush_semaphore` has its own internal `Mutex`/`Condvar` and is never
///   wrapped in an outer lock.
/// - `file_loggers` is a single `Mutex`: the logging thread is its only
///   regular user (locked once per batch and once per flush), so finer
///   locking would buy nothing. `add_context`/`remove_context` and the
///   panic-hook fallback lock it briefly and rarely.
/// - `sinks` is only mutated by rare, non-hot-path calls (`add_sink`); the
///   logging thread only ever takes a *read* lock on it.
struct SharedLoggerState {
    /// Signaled once a flush completes, so callers of `wait_for_flush` /
    /// `wait_for_flush_timeout` can block until logs are durably written.
    flush_semaphore: SemaphoreLite,

    config: LoggerConfig,

    /// Map of context log files. ID -> Log file
    #[cfg(feature = "file")]
    file_loggers: Mutex<FileLoggers>,

    /// Additional log sinks
    sinks: RwLock<Vec<Box<dyn LogCallback>>>,
}
/// Producer-side handle. Cloneable, thread-safe — every caller of
/// `queue_log`/`queue_logs` sends through this.
#[derive(Clone)]
pub struct Logger {
    log_sender: Sender<LogData>,
    shared: Arc<SharedLoggerState>,
}

/// Consumer-side handle. Only the dedicated logging thread reads from
/// this.
struct LoggerThread {
    log_receiver: Receiver<LogData>,

    shared: Arc<SharedLoggerState>,
}

impl LoggerConfig {
    pub fn init(self, install_panic_hook: bool) -> Result<ThreadSafeLoggerThread> {
        // Unbounded MPSC: many producer threads/call sites, one consumer
        // (the logging thread). Swap for `channel::bounded(n)` if you want
        // backpressure instead of unbounded growth under a log storm.
        let (log_sender, log_receiver) = channel::unbounded();

        #[cfg(feature = "file")]
        let file_loggers = {
            let log_path = self.log_path.clone();
            FileLoggers::new(log_path.clone(), self.context_log_path.clone()).map_err(|e| {
                LoggerError::IoSpecificError(
                    e,
                    Some("Unable to create file loggers".to_string()),
                    log_path,
                )
            })?
        };

        let shared = Arc::new(SharedLoggerState {
            flush_semaphore: SemaphoreLite::new(),
            #[cfg(feature = "file")]
            file_loggers: Mutex::new(file_loggers),
            sinks: RwLock::new(Vec::new()),
            config: self,
        });

        let logger = Logger {
            log_sender,
            shared: Arc::clone(&shared),
        };

        let logger_thread = LoggerThread {
            log_receiver,
            shared,
        };

        #[cfg(all(feature = "tracing", target_os = "android"))]
        paranoid_android::init("paper");

        if install_panic_hook {
            std::panic::set_hook(panic_hook(true, true, logger.clone()));
        }

        // `tracing`'s default dispatcher is thread-local, so the dedicated
        // logging thread spawned below wouldn't otherwise see whatever
        // subscriber (e.g. a test's `#[traced_test]` capture) is active on
        // the calling thread. Capture it here and re-install it there.
        #[cfg(feature = "tracing")]
        let dispatch = tracing::dispatcher::get_default(|d| d.clone());

        // The thread deliberately holds no `Logger` (and so no `Sender`):
        // once every `Logger` is dropped the channel disconnects and the
        // thread flushes and exits on its own.
        thread::spawn(move || {
            #[cfg(feature = "tracing")]
            let _dispatch_guard = tracing::dispatcher::set_default(&dispatch);

            logger_thread.log_thread();
        });

        Ok(logger)
    }
}

impl Logger {
    /// Waits indefinitely until the next flush completes.
    pub fn wait_for_flush(&self) {
        self.shared.flush_semaphore.wait();
    }
    pub fn wait_for_flush_timeout(&self, duration: Duration) {
        self.shared.flush_semaphore.wait_timeout(duration);
    }

    pub fn config(&self) -> &LoggerConfig {
        &self.shared.config
    }

    /// Registers a new logging context. This is not on the hot path (called
    /// rarely, e.g. once per mod/module at startup), so blocking on the
    /// `file_loggers` lock is fine.
    pub fn add_context(&self, tag: &PaperStr) -> Result<()> {
        #[cfg(feature = "file")]
        {
            self.shared.file_loggers.lock().add_context(tag)?;
        }
        #[cfg(not(feature = "file"))]
        let _ = tag;

        Ok(())
    }
    pub fn remove_context(&self, tag: &PaperStr) {
        #[cfg(feature = "file")]
        {
            self.shared.file_loggers.lock().remove_context(tag);
        }
        #[cfg(not(feature = "file"))]
        let _ = tag;
    }

    pub fn add_sink<F>(&self, sink: F)
    where
        F: LogCallback + 'static,
    {
        self.shared.sinks.write().push(Box::new(sink));
    }

    pub fn get_sinks(&self) -> &RwLock<Vec<Box<dyn LogCallback>>> {
        &self.shared.sinks
    }

    /// Queues a log entry to be written by the logging thread.
    /// This is thread-safe; `Sender<T>` is `Clone + Send + Sync`. This is the
    /// hot path: no lock is taken, only a channel send.
    pub fn queue_log(&self, log_data: LogData) {
        // If the consumer has been dropped (shutdown), just drop the log
        // rather than panicking the caller.
        let _ = self.log_sender.send(log_data);
    }

    /// Queues multiple log entries to be written by the logging thread.
    /// This is thread-safe.
    pub fn queue_logs(&self, log_data: impl Iterator<Item = LogData>) {
        for log_data in log_data {
            let _ = self.log_sender.send(log_data);
        }
    }

    /// Approximate number of log entries not yet picked up by the logging
    /// thread. Mainly useful for tests/inspection.
    pub fn queue_len(&self) -> usize {
        self.log_sender.len()
    }

    #[cfg(feature = "backtrace")]
    #[inline(always)]
    pub fn backtrace(&self) -> Result<()> {
        use std::backtrace::Backtrace;

        let backtrace = Backtrace::capture();
        let backtrace_str = format!("{backtrace:?}").into();

        self.queue_log(LogData {
            level: LogLevel::Error,
            tag: None,
            message: backtrace_str,
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..LogData::default()
        });

        Ok(())
    }
}

/// Empties `v` and hands back its allocation retyped for a different
/// lifetime, so a buffer of borrowed chunks can be reused across batches
/// even though each batch borrows from a different `queue` generation.
///
/// Relies on `Vec`'s in-place `collect` (same size/alignment element type),
/// which reuses the source allocation; `recycled_vec_keeps_capacity` below
/// guards that assumption.
fn recycle_vec<'b>(mut v: Vec<LogDataChunk<'_>>) -> Vec<LogDataChunk<'b>> {
    v.clear();
    v.into_iter().map(|_| unreachable!()).collect()
}

impl LoggerThread {
    /// The main logging thread function.
    /// Blocks on the channel for the next entry, then drains whatever else
    /// has queued up (non-blocking) so writes happen in batches.
    /// Returns when all `Sender` handles are dropped (channel disconnected).
    ///
    /// Output errors never end the loop: they're reported through
    /// [`ErrorReporter`] and logging carries on, since a logging thread that
    /// silently exits leaves producers filling an unbounded channel nobody
    /// reads.
    fn log_thread(&self) {
        //TODO: Use config max buffer count to limit log batch size
        let mut logged = 0;
        let mut queue = Vec::with_capacity(1024);
        let mut chunks_buf: Vec<LogDataChunk<'static>> = Vec::with_capacity(1024);
        let mut errors = ErrorReporter::default();

        loop {
            queue.clear();
            // Block until at least one log arrives. Err means every Sender
            // was dropped, i.e. clean shutdown — exit the loop.
            let Ok(first) = self.log_receiver.recv() else {
                break;
            };

            queue.push(first);

            // Drain anything else already queued, without blocking, so we
            // batch writes instead of doing one syscall per log line.
            while let Ok(log) = self.log_receiver.try_recv() {
                queue.push(log);
            }

            logged += queue.len();

            let max_str_len = self.shared.config.max_string_len;
            // Collect the split logs so file writes can be batched under one
            // lock, reusing last batch's allocation.
            let mut logs_vec = recycle_vec(std::mem::take(&mut chunks_buf));
            logs_vec.extend(logger_chunk::split_str_into_chunks(&queue, max_str_len));

            // Batch file writes under a single lock acquisition.
            #[cfg(feature = "file")]
            errors.check(
                ErrorSource::File,
                crate::logger::sinks::file_logger::do_log_batch(
                    &logs_vec,
                    &mut self.shared.file_loggers.lock(),
                ),
            );

            // Lock the sinks list once for the whole batch rather than once
            // per log line.
            #[cfg(feature = "sinks")]
            let sinks_guard = self.shared.sinks.read();

            // Same for stdout: one lock and one buffered write per batch
            // instead of a lock + line-buffered write per `println!`.
            #[cfg(feature = "stdout")]
            let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());

            // Call non-file backends per log (these are typically cheaper and may
            // require per-log handling).
            #[allow(unused_variables)] // with no per-line backends enabled
            for &log in &logs_vec {
                #[cfg(all(target_os = "android", feature = "logcat"))]
                errors.check(
                    ErrorSource::Logcat,
                    crate::logger::sinks::logcat_logger::do_log(log),
                );

                #[cfg(feature = "stdout")]
                errors.check(
                    ErrorSource::Stdout,
                    crate::logger::sinks::stdout_logger::do_log(log, &mut stdout),
                );

                #[cfg(feature = "sinks")]
                crate::logger::sinks::sink_logger::do_log(log, &sinks_guard, |e| {
                    errors.error(ErrorSource::Sink, &e)
                });

                #[cfg(feature = "tracing")]
                errors.check(
                    ErrorSource::Tracing,
                    crate::logger::sinks::tracing_logger::do_log(log),
                );
            }

            #[cfg(feature = "stdout")]
            errors.check(ErrorSource::Stdout, std::io::Write::flush(&mut stdout));

            #[cfg(feature = "sinks")]
            drop(sinks_guard);

            chunks_buf = recycle_vec(logs_vec);

            // if no more logs in the pipeline, flush it
            // or flush every 100 lines to avoid missing flushes
            if self.log_receiver.is_empty() || logged >= 100 {
                errors.check(ErrorSource::Flush, self.flush());
                logged = 0;
            }

            errors.end_batch();
        }

        // Channel disconnected: make sure nothing is left in the buffers.
        errors.check(ErrorSource::Flush, self.flush());
    }

    /// Flushes all log files.
    /// This is called after all logs in the queue have been processed.
    /// This function is thread-safe.
    fn flush(&self) -> std::result::Result<(), std::io::Error> {
        // flush file
        #[cfg(feature = "file")]
        {
            self.shared.file_loggers.lock().flush()?;
        }

        // signal flush complete
        self.shared.flush_semaphore.signal();
        Ok(())
    }
}

/// How long the synchronous [`do_log`] path waits for a lock before giving
/// up. `do_log` runs from the panic hook, which can fire on the logging
/// thread itself while it already holds `file_loggers` (non-reentrant) or a
/// read guard on `sinks` (a second read would queue behind a waiting
/// writer). Blocking there would deadlock, so it times out instead.
#[cfg(any(feature = "file", feature = "sinks"))]
const SYNC_LOG_LOCK_TIMEOUT: Duration = Duration::from_millis(100);

/// Logs a single, unchunked log entry to all enabled backends, synchronously.
/// Used for internal logger errors (panic hook) rather than the hot path,
/// which chunks messages via `LogDataChunk` instead.
///
/// The file output is flushed immediately, since a panic may be followed by
/// an abort. If a lock can't be taken within [`SYNC_LOG_LOCK_TIMEOUT`], that
/// backend is skipped and the line goes to stderr (and logcat) instead.
#[allow(unused_variables)] // with no backends enabled
pub fn do_log(log: &LogData, logger: &Logger) -> Result<()> {
    #[allow(unused_imports)]
    use crate::logger::sinks::*;

    // Every backend below takes a `LogDataChunk`; build one borrowing the
    // whole message rather than allocating an owned copy.
    #[allow(unused_variables)]
    let chunk = LogDataChunk {
        original: log,
        message: log.message.as_bstr(),
    };

    #[cfg(all(target_os = "android", feature = "logcat"))]
    logcat_logger::do_log(chunk)?;

    #[cfg(feature = "file")]
    match logger
        .shared
        .file_loggers
        .try_lock_for(SYNC_LOG_LOCK_TIMEOUT)
    {
        Some(mut files) => {
            file_logger::do_log(log, &mut files)?;
            files.flush()?;
        }
        None => report_internal(format_args!("{chunk}")),
    }

    #[cfg(feature = "stdout")]
    stdout_logger::do_log(chunk, &mut std::io::stdout().lock())?;

    #[cfg(feature = "sinks")]
    if let Some(sinks) = logger.shared.sinks.try_read_for(SYNC_LOG_LOCK_TIMEOUT) {
        let mut first_error = None;
        sink_logger::do_log(chunk, &sinks, |e| {
            first_error.get_or_insert(e);
        });
        if let Some(e) = first_error {
            return Err(e);
        }
    }

    #[cfg(feature = "tracing")]
    tracing_logger::do_log(chunk)?;

    Ok(())
}

/// Returns a panic handler, optionally with backtrace and spantrace capture.
pub fn panic_hook(
    backtrace: bool,
    _spantrace: bool,
    logger_thread: ThreadSafeLoggerThread,
) -> Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static> {
    // Mostly taken from https://doc.rust-lang.org/src/std/panicking.rs.html
    Box::new(move |info| {
        let location = info.location().unwrap();
        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        };

        log_data_to!(
            &logger_thread,
            LogLevel::Error,
            Some("panic".into()),
            "panicked at '{msg}', {location}"
        );
        if backtrace {
            log_data_to!(
                &logger_thread,
                LogLevel::Error,
                Some("panic".into()),
                "{:?}",
                Backtrace::force_capture()
            );
        }

        #[cfg(feature = "tracing")]
        if _spantrace {
            use tracing_error::SpanTrace;

            log_data_to!(
                &logger_thread,
                LogLevel::Error,
                Some("panic".into()),
                "{:?}",
                SpanTrace::capture()
            );
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recycled_vec_keeps_capacity() {
        let log = LogData::default();
        let mut v: Vec<LogDataChunk<'_>> = Vec::with_capacity(64);
        v.push(LogDataChunk {
            original: &log,
            message: log.message.as_bstr(),
        });
        let ptr = v.as_ptr() as usize;

        let recycled: Vec<LogDataChunk<'static>> = recycle_vec(v);
        assert!(recycled.is_empty());
        assert!(recycled.capacity() >= 64);
        assert_eq!(recycled.as_ptr() as usize, ptr);
    }
}

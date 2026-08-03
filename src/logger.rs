use std::{
    backtrace::Backtrace, io::Write, panic::PanicHookInfo, sync::Arc, thread, time::Duration,
};

#[cfg(feature = "file")]
use crate::types::file_logger_map::FileLoggers;
use crate::{
    semaphore_lite::SemaphoreLite,
    types::config::LoggerConfig,
    types::log_level::LogLevel,
    types::{
        logger_chunk::{self, LogDataChunk},
        LogData, PaperStr,
    },
    LoggerError, Result,
};
use bstr::ByteSlice;
use chrono::Local;
use crossbeam::channel::{self, Receiver, Sender};
use parking_lot::RwLock;

pub mod sinks;

pub trait LogCallback = Fn(LogDataChunk<'_>) -> Result<()> + Send + Sync;

// Helper macro to reduce repetition when constructing `LogData` and calling `do_log`.
// The macro performs `format!` internally — pass format-style arguments directly.
// Usage: log_data_to!(&logger, LogLevel::Error, Some("tag".to_string()), "msg: {}", val);
macro_rules! log_data_to {
    ($logger:expr, $level:expr, $tag:expr, $($arg:tt)+) => {{
        let message = format!($($arg)+).into();
        let _ = do_log(
            &LogData {
                level: $level,
                tag: $tag,
                message,
                timestamp: Local::now(),
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
/// - `file_loggers` and `sinks` are only mutated by rare, non-hot-path calls
///   (`add_context`/`remove_context`/`add_sink`); the logging thread only
///   ever takes a *read* lock on them (each individual file inside
///   `FileLoggers` has its own `Mutex`, so a shared read lock is enough to
///   write to it).
struct SharedLoggerState {
    /// Signaled once a flush completes, so callers of `wait_for_flush` /
    /// `wait_for_flush_timeout` can block until logs are durably written.
    flush_semaphore: SemaphoreLite,

    config: LoggerConfig,

    /// Map of context log files. ID -> Log file
    #[cfg(feature = "file")]
    file_loggers: RwLock<FileLoggers>,

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
            file_loggers: RwLock::new(file_loggers),
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

        #[cfg(feature = "tracing")]
        {
            use cfg_if::cfg_if;
            cfg_if! {
                if #[cfg(all(target_os = "android"))] {
                    paranoid_android::init("paper");
                }
            }
        }

        if install_panic_hook {
            std::panic::set_hook(panic_hook(true, true, logger.clone()));
        }

        // `tracing`'s default dispatcher is thread-local, so the dedicated
        // logging thread spawned below wouldn't otherwise see whatever
        // subscriber (e.g. a test's `#[traced_test]` capture) is active on
        // the calling thread. Capture it here and re-install it there.
        #[cfg(feature = "tracing")]
        let dispatch = tracing::dispatcher::get_default(|d| d.clone());

        let logger_for_thread = logger.clone();

        thread::spawn(move || {
            #[cfg(feature = "tracing")]
            let _dispatch_guard = tracing::dispatcher::set_default(&dispatch);

            let result = logger_thread.log_thread();

            // handle log thread dying
            if let Err(e) = result {
                let _ = logger_thread.flush();

                log_data_to!(
                    &logger_for_thread,
                    LogLevel::Error,
                    Some("Paper2".into()),
                    "Error occurred in logging thread: {e}"
                );
            }
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
    /// `file_loggers` write lock is fine.
    pub fn add_context(&self, tag: &PaperStr) -> Result<()> {
        #[cfg(feature = "file")]
        {
            let mut file_loggers = self.shared.file_loggers.write();
            file_loggers.add_context(tag)?;
        }

        Ok(())
    }
    pub fn remove_context(&self, tag: &PaperStr) {
        #[cfg(feature = "file")]
        {
            let mut file_loggers = self.shared.file_loggers.write();
            file_loggers.remove_context(tag);
        }
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

impl LoggerThread {
    /// The main logging thread function.
    /// Blocks on the channel for the next entry, then drains whatever else
    /// has queued up (non-blocking) so writes happen in batches.
    /// Returns when all `Sender` handles are dropped (channel disconnected).
    fn log_thread(&self) -> Result<()> {
        //TODO: Use config max buffer count to limit log batch size
        let mut logged = 0;
        let mut queue = Vec::with_capacity(1024);

        loop {
            queue.clear();
            // Block until at least one log arrives. Err means every Sender
            // was dropped, i.e. clean shutdown — exit the loop.
            let first = match self.log_receiver.recv() {
                Ok(log) => log,
                Err(_) => break,
            };

            queue.push(first);

            // Drain anything else already queued, without blocking, so we
            // batch writes instead of doing one syscall per log line.
            while let Ok(log) = self.log_receiver.try_recv() {
                queue.push(log);
            }

            logged += queue.len();

            let max_str_len = self.shared.config.max_string_len;
            // collect the split logs into a vec so we can batch file writes
            let logs_vec: Vec<_> =
                logger_chunk::split_str_into_chunks(&queue, max_str_len).collect();

            // Batch file writes under a single read lock (each file has its
            // own internal `Mutex`) to reduce lock-acquisition overhead.
            #[cfg(feature = "file")]
            {
                crate::logger::sinks::file_logger::do_log_batch(
                    &logs_vec,
                    &self.shared.file_loggers.read(),
                )?;
            }

            // Lock the sinks list once for the whole batch rather than once
            // per log line.
            #[cfg(feature = "sinks")]
            let sinks_guard = self.shared.sinks.read();

            // Call non-file backends per log (these are typically cheaper and may
            // require per-log handling).
            for log in logs_vec {
                #[cfg(all(target_os = "android", feature = "logcat"))]
                crate::logger::sinks::logcat_logger::do_log(log)?;

                #[cfg(feature = "stdout")]
                crate::logger::sinks::stdout_logger::do_log(log);

                #[cfg(feature = "sinks")]
                crate::logger::sinks::sink_logger::do_log(log, &sinks_guard)?;

                #[cfg(feature = "tracing")]
                crate::logger::sinks::tracing_logger::do_log(log)?;
            }

            // if no more logs in the pipeline, flush it
            // or flush every 100 lines to avoid missing flushes
            if self.log_receiver.is_empty() || logged >= 100 {
                self.flush()
                    .map_err(|e| LoggerError::FlushError(Box::new(e)))?;
                logged = 0;
            }
        }

        Ok(())
    }

    /// Flushes all log files.
    /// This is called after all logs in the queue have been processed.
    /// This function is thread-safe.
    fn flush(&self) -> std::result::Result<(), std::io::Error> {
        // flush file
        #[cfg(feature = "file")]
        {
            // Only a read lock is needed: each file is independently
            // synchronized via its own `Mutex`.
            let file_loggers = self.shared.file_loggers.read();
            file_loggers.get_global_file().lock().flush()?;
            file_loggers
                .get_context_map()
                .values()
                .try_for_each(|file| file.lock().flush())?;
        }

        // signal flush complete
        self.shared.flush_semaphore.signal();
        Ok(())
    }
}

/// Logs a single, unchunked log entry to all enabled backends. Used for
/// internal logger errors (panic hook, logging-thread failure) rather than
/// the hot path, which chunks messages via `LogDataChunk` instead.
pub fn do_log(log: &LogData, logger: &Logger) -> Result<()> {
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
    file_logger::do_log(log, &logger.shared.file_loggers.read())?;

    #[cfg(feature = "stdout")]
    stdout_logger::do_log(chunk);

    #[cfg(feature = "sinks")]
    sink_logger::do_log(chunk, &logger.shared.sinks.read())?;

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

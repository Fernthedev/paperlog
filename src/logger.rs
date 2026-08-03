use std::{
    backtrace::Backtrace,
    fs::{self, File},
    io::{BufWriter, Write},
    panic::PanicHookInfo,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crate::{
    semaphore_lite::SemaphoreLite,
    types::config::LoggerConfig,
    types::log_level::LogLevel,
    types::{
        logger_chunk::{self, LogDataChunk},
        LogData, PaperStr, PaperString,
    },
    vec_pool::VecPool,
    LoggerError, Result,
};
use bstr::ByteSlice;
use chrono::Local;
use crossbeam::channel::{self, Receiver, Sender};
use parking_lot::RwLock;

pub mod sinks;

#[cfg(feature = "file")]
use rustc_hash::FxHashMap;

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

pub type ThreadSafeLoggerThread = Arc<RwLock<LoggerThreadCtx>>;

pub struct LoggerThreadCtx {
    pub config: LoggerConfig,

    /// Producer-side handle. Cloneable, thread-safe — every caller of
    /// `queue_log`/`queue_logs` sends through this.
    log_sender: Sender<LogData>,

    /// Consumer-side handle. Held here only until `init()` moves it into the
    /// dedicated logging thread; `None` afterwards.
    log_receiver: Option<Receiver<LogData>>,

    /// Signaled once a flush completes, so callers of `wait_for_flush` /
    /// `wait_for_flush_timeout` can block until logs are durably written.
    flush_semaphore: Arc<SemaphoreLite>,

    /// Whether the logger has been initialized
    inited: AtomicBool,

    /// Global log file
    #[cfg(feature = "file")]
    pub(super) global_file: BufWriter<File>,

    /// Map of context log files. ID -> Log file
    #[cfg(feature = "file")]
    pub(super) context_map: FxHashMap<PaperString, BufWriter<File>>,

    /// Additional log sinks
    pub(super) sinks: Vec<Box<dyn LogCallback>>,
}

impl LoggerThreadCtx {
    pub fn new(config: LoggerConfig, log_path: PathBuf) -> Result<Self> {
        // Unbounded MPSC: many producer threads/call sites, one consumer
        // (the logging thread). Swap for `channel::bounded(n)` if you want
        // backpressure instead of unbounded growth under a log storm.
        let (log_sender, log_receiver) = channel::unbounded();
        let flush_semaphore = Arc::new(SemaphoreLite::new());

        #[cfg(feature = "file")]
        let global_file = {
            fs::create_dir_all(&config.context_log_path).map_err(|e| {
                LoggerError::IoSpecificError(
                    e,
                    Some("Unable to make logging directory for contexts".to_string()),
                    config.context_log_path.clone(),
                )
            })?;

            if let Some(parent) = log_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    LoggerError::IoSpecificError(
                        e,
                        Some("Unable to make logging directory for global".to_string()),
                        parent.to_path_buf(),
                    )
                })?;
            }

            let inner = File::create(&log_path).map_err(|e| {
                LoggerError::IoSpecificError(
                    e,
                    Some("Unable to create global file".to_string()),
                    log_path,
                )
            })?;
            BufWriter::new(inner)
        };

        Ok(LoggerThreadCtx {
            config,
            log_sender,
            log_receiver: Some(log_receiver),
            flush_semaphore,
            inited: AtomicBool::new(false),

            #[cfg(feature = "file")]
            global_file,

            #[cfg(feature = "file")]
            context_map: Default::default(),

            sinks: Vec::new(),
        })
    }

    pub fn init(mut self, install_panic_hook: bool) -> Result<ThreadSafeLoggerThread> {
        if self.inited.load(Ordering::SeqCst) {
            return Err(LoggerError::AlreadyInitialized);
        }

        self.inited.store(true, Ordering::SeqCst);

        // Move the receiver out before `self` is consumed into the Arc<RwLock<..>>.
        // There is exactly one consumer, so this must succeed on first init.
        let log_receiver = self
            .log_receiver
            .take()
            .expect("log receiver already taken; init() called more than once?");
        let flush_semaphore_clone = Arc::clone(&self.flush_semaphore);
        let thread_safe_self: Arc<RwLock<LoggerThreadCtx>> = Arc::new(self.into());
        let thread_safe_self_clone = Arc::clone(&thread_safe_self);

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
            std::panic::set_hook(panic_hook(true, true, thread_safe_self.clone()));
        }

        // `tracing`'s default dispatcher is thread-local, so the dedicated
        // logging thread spawned below wouldn't otherwise see whatever
        // subscriber (e.g. a test's `#[traced_test]` capture) is active on
        // the calling thread. Capture it here and re-install it there.
        #[cfg(feature = "tracing")]
        let dispatch = tracing::dispatcher::get_default(|d| d.clone());

        thread::spawn(move || {
            #[cfg(feature = "tracing")]
            let _dispatch_guard = tracing::dispatcher::set_default(&dispatch);

            let result = Self::log_thread(
                log_receiver,
                flush_semaphore_clone.clone(),
                thread_safe_self_clone.clone(),
            );

            // handle log thread dying
            if let Err(e) = result {
                let _ = Self::flush(&thread_safe_self_clone, &flush_semaphore_clone);

                log_data_to!(
                    &thread_safe_self_clone,
                    LogLevel::Error,
                    Some("Paper2".into()),
                    "Error occurred in logging thread: {e}"
                );
            }
        });

        Ok(thread_safe_self)
    }

    pub fn is_inited(&self) -> &AtomicBool {
        &self.inited
    }

    pub fn get_sinks(&self) -> &Vec<Box<dyn LogCallback>> {
        &self.sinks
    }

    /// Queues a log entry to be written by the logging thread.
    /// This is thread-safe; `Sender<T>` is `Clone + Send + Sync`.
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

    pub fn add_context(&mut self, tag: &PaperStr) -> Result<()> {
        #[cfg(feature = "file")]
        {
            // `to_path_lossy()` is zero-copy on unix (raw bytes reinterpreted as
            // `OsStr`), so this doesn't require `tag` to be valid UTF-8.
            let log_path = self
                .config
                .context_log_path
                .join(tag.to_path_lossy())
                .with_extension("log");
            let file = File::create(&log_path).map_err(|e| {
                LoggerError::IoSpecificError(
                    e,
                    Some("Unable to create context file".to_string()),
                    log_path,
                )
            })?;
            let file = BufWriter::new(file);

            self.context_map.insert(tag.to_owned(), file);
        }

        Ok(())
    }
    pub fn remove_context(&mut self, tag: &PaperStr) {
        #[cfg(feature = "file")]
        {
            self.context_map.remove(tag);
        }
    }

    pub fn add_sink<F>(&mut self, sink: F)
    where
        F: LogCallback + 'static,
    {
        self.sinks.push(Box::new(sink));
    }

    /// The main logging thread function.
    /// Blocks on the channel for the next entry, then drains whatever else
    /// has queued up (non-blocking) so writes happen in batches.
    /// Returns when all `Sender` handles are dropped (channel disconnected).
    fn log_thread(
        log_receiver: Receiver<LogData>,
        flush_semaphore: Arc<SemaphoreLite>,
        logger_thread: Arc<RwLock<LoggerThreadCtx>>,
    ) -> Result<()> {
        //TODO: Use config max buffer count to limit log batch size
        let mut log_pool: VecPool<LogData> = VecPool::with_initial_amount(2, 1024);
        let mut logged = 0;

        loop {
            // Block until at least one log arrives. Err means every Sender
            // was dropped, i.e. clean shutdown — exit the loop.
            let first = match log_receiver.recv() {
                Ok(log) => log,
                Err(_) => break,
            };

            let mut queue = log_pool.take_vec();
            queue.clear();
            queue.push(first);

            // Drain anything else already queued, without blocking, so we
            // batch writes instead of doing one syscall per log line.
            while let Ok(log) = log_receiver.try_recv() {
                queue.push(log);
            }

            logged += queue.len();

            let max_str_len = logger_thread.read().config.max_string_len;
            // collect the split logs into a vec so we can batch file writes
            let logs_vec: Vec<_> =
                logger_chunk::split_str_into_chunks(&queue, max_str_len).collect();

            // Batch file writes under a single write lock to reduce overhead
            #[cfg(feature = "file")]
            {
                // write files in batch

                crate::logger::sinks::file_logger::do_log_batch(&logs_vec, &logger_thread)?;
            }

            // Call non-file backends per log (these are typically cheaper and may
            // require per-log handling).
            for log in logs_vec {
                #[cfg(all(target_os = "android", feature = "logcat"))]
                crate::logger::sinks::logcat_logger::do_log(log)?;

                #[cfg(feature = "stdout")]
                crate::logger::sinks::stdout_logger::do_log(log);

                #[cfg(feature = "sinks")]
                crate::logger::sinks::sink_logger::do_log(log, &logger_thread)?;

                #[cfg(feature = "tracing")]
                crate::logger::sinks::tracing_logger::do_log(log)?;
            }

            // if no more logs in the pipeline, flush it
            // or flush every 100 lines to avoid missing flushes
            if log_receiver.is_empty() || logged >= 100 {
                Self::flush(&logger_thread, &flush_semaphore)
                    .map_err(|e| LoggerError::FlushError(Box::new(e)))?;
                logged = 0;
            }
        }

        Ok(())
    }

    /// Flushes all log files.
    /// This is called after all logs in the queue have been processed.
    /// This function is thread-safe.
    fn flush(
        logger_thread: &Arc<RwLock<LoggerThreadCtx>>,
        flush_semaphore: &Arc<SemaphoreLite>,
    ) -> std::result::Result<(), std::io::Error> {
        // flush file
        #[cfg(feature = "file")]
        {
            let mut logger_thread = logger_thread.write();
            logger_thread.global_file.flush()?;
            logger_thread
                .context_map
                .values_mut()
                .try_for_each(|file| file.flush())?;
        }

        // signal flush complete
        flush_semaphore.signal();
        Ok(())
    }

    /// Waits indefinitely until the next flush completes.
    pub fn wait_for_flush(&self) {
        self.flush_semaphore.wait();
    }
    pub fn wait_for_flush_timeout(&self, duration: Duration) {
        self.flush_semaphore.wait_timeout(duration);
    }
}

/// Logs a single, unchunked log entry to all enabled backends. Used for
/// internal logger errors (panic hook, logging-thread failure) rather than
/// the hot path, which chunks messages via `LogDataChunk` instead.
pub fn do_log(log: &LogData, logger_thread: &RwLock<LoggerThreadCtx>) -> Result<()> {
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
    file_logger::do_log(log, logger_thread)?;

    #[cfg(feature = "stdout")]
    stdout_logger::do_log(chunk);

    #[cfg(feature = "sinks")]
    sink_logger::do_log(chunk, logger_thread)?;

    #[cfg(feature = "tracing")]
    tracing_logger::do_log(chunk)?;

    Ok(())
}

/// Returns a panic handler, optionally with backtrace and spantrace capture.
pub fn panic_hook(
    backtrace: bool,
    spantrace: bool,
    logger_thread: Arc<RwLock<LoggerThreadCtx>>,
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
        if spantrace {
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

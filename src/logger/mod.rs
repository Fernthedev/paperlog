use std::{
    backtrace::Backtrace,
    collections::HashMap,
    fs::{self, File},
    io::{BufWriter, Write},
    panic::PanicHookInfo,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    thread,
    time::Duration,
};

use crate::{log_level::LogLevel, semaphore_lite::SemaphoreLite, LoggerError, Result};
use chrono::Local;
use itertools::Itertools;

#[cfg(all(target_os = "android", feature = "logcat"))]
pub mod logcat_logger;

#[cfg(feature = "file")]
pub mod file_logger;

#[cfg(feature = "stdout")]
pub mod stdout_logger;

#[cfg(feature = "sinks")]
pub mod sink_logger;

#[cfg(feature = "tracing")]
pub mod tracing_logger;

mod log_data;
pub use log_data::LogData;

pub trait LogCallback: Fn(&LogData) -> Result<()> + Send + Sync {}

pub type ThreadSafeLoggerThread = Arc<RwLock<LoggerThread>>;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub max_string_len: usize,
    pub log_max_buffer_count: usize,
    pub line_end: char,

    #[cfg(feature = "file")]
    pub context_log_path: PathBuf,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        LoggerConfig {
            max_string_len: 1024,
            log_max_buffer_count: 100,
            line_end: '\n',

            #[cfg(feature = "file")]
            context_log_path: PathBuf::from("./logs"),
        }
    }
}

pub struct LoggerThread {
    pub config: LoggerConfig,

    log_queue: Arc<(SemaphoreLite, Mutex<Vec<LogData>>)>,
    flush_semaphore: Arc<SemaphoreLite>,

    inited: AtomicBool,

    #[cfg(feature = "file")]
    global_file: BufWriter<File>,

    #[cfg(feature = "file")]
    context_map: HashMap<String, BufWriter<File>>,

    sinks: Vec<Box<dyn LogCallback>>,
}

impl LoggerThread {
    pub fn new(config: LoggerConfig, log_path: PathBuf) -> Result<Self> {
        let log_queue = Arc::new((
            SemaphoreLite::new(),
            Mutex::new(Vec::with_capacity(config.log_max_buffer_count)),
        ));
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

        Ok(LoggerThread {
            config,
            log_queue,
            flush_semaphore,
            inited: AtomicBool::new(false),

            #[cfg(feature = "file")]
            global_file,

            #[cfg(feature = "file")]
            context_map: HashMap::new(),

            sinks: Vec::new(),
        })
    }

    pub fn init(self, install_panic_hook: bool) -> Result<ThreadSafeLoggerThread> {
        if self.inited.load(Ordering::SeqCst) {
            return Err(LoggerError::AlreadyInitialized);
        }

        self.inited.store(true, Ordering::SeqCst);

        let log_queue_clone = Arc::clone(&self.log_queue);
        let flush_semaphore_clone = Arc::clone(&self.flush_semaphore);
        let thread_safe_self: Arc<RwLock<LoggerThread>> = Arc::new(self.into());
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

        thread::spawn(move || {
            let result = Self::log_thread(
                log_queue_clone,
                flush_semaphore_clone.clone(),
                thread_safe_self_clone.clone(),
            );

            // handle log thread dying
            if let Err(e) = result {
                let _ = Self::flush(&thread_safe_self_clone, &flush_semaphore_clone);

                let _ = do_log(
                    LogData {
                        level: LogLevel::Error,
                        message: format!("Error occurred in logging thread: {e}"),
                        tag: Some("Paper2".to_string()),
                        timestamp: Local::now(),
                        file: file!().to_string(),
                        line: line!(),
                        column: column!(),
                        function_name: None,
                    },
                    &thread_safe_self_clone,
                );
            }
        });

        Ok(thread_safe_self)
    }

    pub fn is_inited(&self) -> &AtomicBool {
        &self.inited
    }

    pub fn get_queue(&self) -> &Mutex<Vec<LogData>> {
        &self.log_queue.1
    }

    pub fn get_sinks(&self) -> &Vec<Box<dyn LogCallback>> {
        &self.sinks
    }

    pub fn queue_log(&self, log_data: LogData) {
        let (sempahore, queue) = self.log_queue.as_ref();

        queue.lock().unwrap().push(log_data);
        sempahore.signal();
    }

    #[cfg(feature = "backtrace")]
    #[inline(always)]
    pub fn backtrace(&self) -> Result<()> {
        use std::backtrace::Backtrace;

        let backtrace = Backtrace::capture();
        let backtrace_str = format!("{:?}", backtrace);

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

    pub fn add_context(&mut self, tag: &str) -> Result<()> {
        #[cfg(feature = "file")]
        {
            let log_path = self.config.context_log_path.join(tag).with_extension("log");
            let file = File::create(&log_path).map_err(|e| {
                LoggerError::IoSpecificError(
                    e,
                    Some("Unable to create context file".to_string()),
                    log_path,
                )
            })?;
            let file = BufWriter::new(file);

            self.context_map.insert(tag.to_string(), file);
        }

        Ok(())
    }
    pub fn remove_context(&mut self, tag: &str) {
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

    fn log_thread(
        log_queue: Arc<(SemaphoreLite, Mutex<Vec<LogData>>)>,
        flush_semaphore: Arc<SemaphoreLite>,
        logger_thread: Arc<RwLock<LoggerThread>>,
    ) -> Result<()> {
        let (log_semaphore_lite, log_mutex) = log_queue.as_ref();

        let mut vec = Vec::with_capacity(100);

        loop {
            // move items from queue to local variable
            // then resize the vec to 100
            // preventing an infinite growing log buffer
            let queue = log_mutex.replace(vec).expect("queue");
            vec = Vec::with_capacity(100);

            // if queue is not empty, write the logs
            if !queue.is_empty() {
                let max_str_len = logger_thread
                    .read()
                    .expect("max_str_len")
                    .config
                    .max_string_len;
                let split_logs = split_str_into_chunks(queue, max_str_len);

                for log in split_logs {
                    do_log(log, &logger_thread)?;
                }
            }

            // if no more logs in the pipeline, flush it
            {
                let locked_list = log_mutex.lock().expect("is_empty");
                let is_empty = locked_list.is_empty();

                if is_empty {
                    Self::flush(&logger_thread, &flush_semaphore)
                        .map_err(|e| LoggerError::FlushError(Box::new(e)))?;

                    // wait for more logs
                    drop(locked_list);
                    log_semaphore_lite.wait();
                }
            }
        }
    }

    fn flush(
        logger_thread: &Arc<RwLock<LoggerThread>>,
        flush_semaphore: &Arc<SemaphoreLite>,
    ) -> std::result::Result<(), std::io::Error> {
        // flush file
        #[cfg(feature = "file")]
        {
            let mut logger_thread = logger_thread.write().unwrap();
            logger_thread.global_file.flush()?;
            logger_thread
                .context_map
                .values_mut()
                .try_for_each(|file| file.flush())?;
        }

        flush_semaphore.signal();
        Ok(())
    }

    ///
    /// Waits indefinitely until the next queue is flushed
    /// May block until a log is called forth
    pub(crate) fn wait_for_flush(&self) {
        self.log_queue.0.wait();
    }
    pub(crate) fn wait_for_flush_timeout(&self, duration: Duration) {
        self.log_queue.0.wait_timeout(duration);
    }
}

/// Split log message by line endings and then split each line into chunks
fn split_str_into_chunks(queue: Vec<LogData>, max_str_len: usize) -> impl Iterator<Item = LogData> {
    queue.into_iter().flat_map(move |log| {
        // split log message by line endings
        log.message
            .split("\n")
            .flat_map(|s| {
                // split string into chunks
                s.chars()
                    .chunks(max_str_len)
                    .into_iter()
                    .map(|chunk| {
                        let chunk = chunk.collect::<String>();
                        LogData {
                            message: chunk,
                            ..log.clone()
                        }
                    })
                    .collect_vec()
            })
            .collect_vec()
    })
}

pub fn do_log(log: LogData, logger_thread: &RwLock<LoggerThread>) -> Result<()> {
    #[cfg(all(target_os = "android", feature = "logcat"))]
    logcat_logger::do_log(&log)?;

    #[cfg(feature = "file")]
    file_logger::do_log(&log, logger_thread)?;

    #[cfg(feature = "stdout")]
    stdout_logger::do_log(&log);

    #[cfg(feature = "sinks")]
    sink_logger::do_log(&log, logger_thread)?;

    #[cfg(feature = "tracing")]
    tracing_logger::do_log(&log)?;

    Ok(())
}

/// Returns a panic handler, optionally with backtrace and spantrace capture.
pub fn panic_hook(
    backtrace: bool,
    spantrace: bool,
    logger_thread: Arc<RwLock<LoggerThread>>,
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

        let _ = do_log(
            LogData {
                level: LogLevel::Error,
                tag: Some("panic".to_string()),
                message: format!("panicked at '{}', {}", msg, location),
                timestamp: Local::now(),
                file: file!().to_string(),
                line: line!(),
                column: column!(),
                function_name: None,
            },
            &logger_thread,
        );
        if backtrace {
            let _ = do_log(
                LogData {
                    level: LogLevel::Error,
                    tag: Some("panic".to_string()),
                    message: format!("{:?}", Backtrace::force_capture()),
                    timestamp: Local::now(),
                    file: file!().to_string(),
                    line: line!(),
                    column: column!(),
                    function_name: None,
                },
                &logger_thread,
            );
        }

        #[cfg(feature = "tracing")]
        if spantrace {
            use tracing_error::SpanTrace;

            let _ = do_log(
                LogData {
                    level: LogLevel::Error,
                    tag: Some("panic".to_string()),
                    message: format!("{:?}", SpanTrace::capture()),
                    timestamp: Local::now(),
                    file: file!().to_string().into(),
                    line: line!(),
                    column: column!(),
                    function_name: None,
                },
                logger_thread.clone(),
            );
        }
    })
}

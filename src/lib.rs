use std::{path::PathBuf, sync::OnceLock};

mod log_level;
mod logger;
mod semaphore_lite;

static LOGGER: OnceLock<ThreadSafeLoggerThread> = OnceLock::new();

#[cfg(feature = "ffi")]
mod ffi;

#[cfg(test)]
mod tests;

pub use logger::{LoggerConfig, LoggerThread, ThreadSafeLoggerThread};

pub type Result<T> = color_eyre::Result<T>;

pub fn get_logger() -> Option<ThreadSafeLoggerThread> {
    LOGGER.get().cloned()
}

pub fn init_logger(config: LoggerConfig, path: PathBuf) -> Result<ThreadSafeLoggerThread> {

    let res = LOGGER
        .get_or_init(|| {
            let logger = LoggerThread::new(config, path).expect("Unable to create logger");
            let thread: std::sync::Arc<std::sync::RwLock<LoggerThread>> =
                logger.init(false).expect("Unable to init logger");

            thread
        })
        .clone();

    Ok(res)
}

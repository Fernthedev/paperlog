#![feature(error_generic_member_access)]

use std::{backtrace, path::PathBuf, sync::OnceLock};

mod log_level;
mod logger;
mod semaphore_lite;

static LOGGER: OnceLock<ThreadSafeLoggerThread> = OnceLock::new();

#[cfg(feature = "ffi")]
mod ffi;

#[cfg(test)]
mod tests;

pub use logger::{do_log, LoggerConfig, LoggerThread, ThreadSafeLoggerThread};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, LoggerError>;

#[derive(Error, Debug)]
pub enum LoggerError {
    #[error("Unknown Error occurred in logging thread: {0}")]
    LogError(String, backtrace::Backtrace),

    #[error("Error occurred during flush in logging thread: {0}")]
    FlushError(
        #[backtrace]
        #[source]
        Box<dyn std::error::Error + Sync + Send>,
    ),

    #[error("IO Error occurred in logging thread: {0}")]
    IoError(
        #[from]
        #[backtrace]
        std::io::Error,
    ),

    #[error("IO Error at {2} occurred in logging thread: Context {1:?} {0}")]
    IoSpecificError(
        #[backtrace]
        #[source]
        std::io::Error,
        Option<String>,
        PathBuf,
    ),

    #[error("Logger already initialized")]
    AlreadyInitialized,
}

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

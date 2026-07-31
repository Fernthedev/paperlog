#![feature(error_generic_member_access)]
#![feature(lock_value_accessors)]
#![feature(trait_alias)]

use std::{backtrace, path::PathBuf, sync::OnceLock};

pub mod log_level;
pub mod logger;
pub mod vec_pool;
pub mod types;

mod semaphore_lite;

static LOGGER: OnceLock<ThreadSafeLoggerThread> = OnceLock::new();

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(test)]
mod tests;

use logger::LoggerConfig;
use thiserror::Error;

use crate::logger::logger_thread_ctx::{LoggerThreadCtx, ThreadSafeLoggerThread};

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

/// Returns a `'static` reference to the global logger, if initialized.
///
/// This borrows straight out of `LOGGER` instead of cloning the `Arc`: since
/// `LOGGER` is itself `'static` and never repopulated once set, the `Arc`'s
/// refcount doesn't need to be bumped just to hand out a reference that
/// already lives for the rest of the program. This is on the hot path for
/// every FFI call, so skipping that atomic increment/decrement pair matters.
#[inline]
pub fn get_logger() -> Option<&'static ThreadSafeLoggerThread> {
    LOGGER.get()
}

pub fn init_logger(config: LoggerConfig, path: PathBuf) -> Result<ThreadSafeLoggerThread> {
    let res = LOGGER
        .get_or_init(|| {
            let logger = LoggerThreadCtx::new(config, path).expect("Unable to create logger");

            logger.init(false).expect("Unable to init logger")
        })
        .clone();

    Ok(res)
}

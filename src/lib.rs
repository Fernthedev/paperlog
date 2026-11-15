use std::{path::PathBuf, sync::OnceLock};

pub mod logger;
mod string_helper;
pub mod types;

mod semaphore_lite;

static LOGGER: OnceLock<ThreadSafeLoggerThread> = OnceLock::new();

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(test)]
mod tests;

use thiserror::Error;

use crate::{logger::ThreadSafeLoggerThread, types::config::LoggerConfig};

pub type Result<T> = std::result::Result<T, LoggerError>;

#[derive(Error, Debug)]
pub enum LoggerError {
    #[error("Unknown Error occurred in logging thread: {0}")]
    LogError(String),

    #[error("Error occurred during flush in logging thread: {0}")]
    FlushError(#[source] Box<dyn std::error::Error + Sync + Send>),

    #[error("IO Error occurred in logging thread: {0}")]
    IoError(#[from] std::io::Error),

    #[error("IO Error at {2} occurred in logging thread: Context {1:?} {0}")]
    IoSpecificError(#[source] std::io::Error, Option<String>, PathBuf),

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

/// Initializes the process-global logger.
///
/// # Errors
/// - [`LoggerError::AlreadyInitialized`] if a global logger already exists.
///   The existing logger is left untouched and `config`/`path` are ignored;
///   use [`get_logger`] to reach it.
/// - Any IO error from creating the log files.
pub fn init_logger(config: LoggerConfig, path: PathBuf) -> Result<ThreadSafeLoggerThread> {
    #[cfg(feature = "file")]
    let config = LoggerConfig {
        log_path: path,
        ..config
    };
    #[cfg(not(feature = "file"))]
    let _ = path;

    if LOGGER.get().is_some() {
        return Err(LoggerError::AlreadyInitialized);
    }

    let logger = config.init(false)?;
    // If another thread won a concurrent init, ours is dropped here: that
    // drops its last `Sender`, so its logging thread sees the channel
    // disconnect and exits on its own.
    LOGGER
        .set(logger.clone())
        .map_err(|_| LoggerError::AlreadyInitialized)?;
    Ok(logger)
}

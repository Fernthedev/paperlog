use std::path::PathBuf;

mod log_level;
mod logger;
mod semaphore_lite;

#[cfg(feature = "ffi")]
mod ffi;

#[cfg(test)]
mod tests;

pub use logger::{LoggerConfig, LoggerThread, ThreadSafeLoggerThread};

pub type Result<T> = color_eyre::Result<T>;

pub fn init_logger(path: PathBuf) -> Result<ThreadSafeLoggerThread> {
    let config = LoggerConfig {
        max_string_len: 1024,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: path.clone(),
    };

    let logger = LoggerThread::new(config, path)?;
    let thread = logger.init(false)?;

    Ok(thread)
}

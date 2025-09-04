use std::path::PathBuf;

use crate::Result;

pub mod logger_thread_ctx;

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

// export LogData for FFI use
mod log_data;
pub use log_data::LogData;

pub trait LogCallback = Fn(&LogData) -> Result<()> + Send + Sync;


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

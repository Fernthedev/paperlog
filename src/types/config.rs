#[cfg(feature = "file")]
use std::path::PathBuf;

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

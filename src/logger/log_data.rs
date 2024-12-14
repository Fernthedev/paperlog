use std::{
    path::PathBuf,
    time::Instant,
};

use crate::log_level::LogLevel;

#[derive(Debug, Clone)]
pub struct LogData {
    pub level: LogLevel,
    pub tag: Option<String>,
    pub message: String,
    pub timestamp: Instant,

    pub file: PathBuf,
    pub line: u32,
}

impl LogData {
    pub fn new(
        level: LogLevel,
        tag: Option<String>,
        message: String,
        file: PathBuf,
        line: u32,
    ) -> Self {
        Self {
            level,
            tag,
            message,
            timestamp: Instant::now(),
            file,
            line,
        }
    }

    pub fn format(&self) -> String {
        format!(
            "{} [{:?}] [{}] {}\n",
            self.level,
            self.timestamp,
            self.tag.as_deref().unwrap_or("default"),
            self.message
        )
    }
}

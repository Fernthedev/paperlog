use std::time::Instant;

use crate::log_level::LogLevel;

pub const DEFAULT_TAG: &str = "GLOBAL";

#[derive(Debug, Clone)]
pub struct LogData {
    pub level: LogLevel,
    pub tag: Option<String>,
    pub message: String,
    pub timestamp: Instant,

    pub file: String,
    pub line: u32,
    pub column: u32,
    pub function_name: Option<String>,
}

impl LogData {
    pub fn new(
        level: LogLevel,
        tag: Option<String>,
        message: String,
        file: String,
        line: u32,
        column: u32,
        function_name: Option<String>,
    ) -> Self {
        Self {
            level,
            tag,
            message,
            timestamp: Instant::now(),
            file,
            line,
            column,
            function_name,
        }
    }

    pub fn format(&self) -> String {
        format!(
            "{} [{:?}] [{}] {file}:{line}:{column}@{function_name} {}\n",
            self.level,
            self.timestamp,
            self.tag.as_deref().unwrap_or(DEFAULT_TAG),
            self.message,
            line = self.line,
            column = self.column,
            file = self.file,
            function_name = self.function_name.as_deref().unwrap_or("default")
        )
    }
}
impl Default for LogData {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            tag: None,
            message: String::new(),
            timestamp: Instant::now(),
            file: String::new(),
            line: 0,
            column: 0,
            function_name: None,
        }
    }
}

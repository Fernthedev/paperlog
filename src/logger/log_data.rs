use chrono::{DateTime, Local};
use std::sync::Arc;

use crate::log_level::LogLevel;

pub const DEFAULT_TAG: &str = "GLOBAL";

#[derive(Debug, Clone)]
pub struct LogData {
    pub level: LogLevel,
    pub tag: Option<Arc<str>>,
    pub message: Arc<str>,
    pub timestamp: DateTime<Local>,

    pub file: Arc<str>,
    pub line: u32,
    pub column: u32,
    pub function_name: Option<Arc<str>>,
}

impl LogData {
    pub fn new(
        level: LogLevel,
        tag: Option<Arc<str>>,
        message: Arc<str>,
        file: Arc<str>,
        line: u32,
        column: u32,
        function_name: Option<Arc<str>>,
    ) -> Self {
        Self {
            level,
            tag,
            message,
            timestamp: Local::now(),
            file,
            line,
            column,
            function_name,
        }
    }

    pub fn format(&self) -> String {
        format!(
            "{level} {time} [{tag}] [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.level,
            time = self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            tag = self.tag.as_deref().unwrap_or(DEFAULT_TAG),
            message = self.message.as_ref(),
            line = self.line,
            column = self.column,
            file = self.file.as_ref(),
            function_name = self.function_name.as_deref().unwrap_or("default")
        )
    }

    pub fn write_to_io(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(
            writer,
            "{level} {time} [{tag}] [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.level.short(),
            time = self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            tag = self.tag.as_deref().unwrap_or(DEFAULT_TAG),
            message = self.message.as_ref(),
            line = self.line,
            column = self.column,
            file = self.file.as_ref(),
            function_name = self.function_name.as_deref().unwrap_or("default")
        )
    }
    pub fn write_compact_to_io(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(
            writer,
            "{level} {time} [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.level.short(),
            time = self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            message = self.message.as_ref(),
            line = self.line,
            column = self.column,
            file = self.file.as_ref(),
            function_name = self.function_name.as_deref().unwrap_or("default")
        )
    }
}
impl Default for LogData {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            tag: None,
            message: Arc::from(""),
            timestamp: Local::now(),
            file: Arc::from(""),
            line: 0,
            column: 0,
            function_name: None,
        }
    }
}

/// Macro that constructs a `LogData` and immediately calls `do_log` with the
/// provided logger reference. Accepts the logger (e.g. `&logger_arc`), level,
/// optional tag (use `Some("tag")` or `None`), and format-style message
/// arguments. Example:
///
/// crate::log_data_to!(&logger, LogLevel::Error, Some("panic"), "oops: {}", e);
#[macro_export]
macro_rules! log_data_to {
    ($logger:expr, $level:expr, $tag:expr, $($arg:tt)+) => {{
        let message = std::sync::Arc::from(format!($($arg)+));
        let tag_arc = $tag.map(|t| std::sync::Arc::from(t));
        let log = $crate::logger::LogData {
            level: $level,
            tag: tag_arc,
            message,
            timestamp: chrono::Local::now(),
            file: std::sync::Arc::from(file!()),
            line: line!(),
            column: column!(),
            function_name: None,
        };

        let _ = $crate::logger::logger_thread_ctx::do_log(log, $logger);
    }};
}

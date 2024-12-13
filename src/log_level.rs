use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum LogLevel {
    INFO,
    WARN,
    ERROR,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::INFO => write!(f, "INFO"),
            LogLevel::WARN => write!(f, "WARN"),
            LogLevel::ERROR => write!(f, "ERROR"),
        }
    }
}

use std::fmt::Display;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
    Crit,
    Off
}

impl LogLevel {
    pub fn short(&self) -> char {
        match self {
            LogLevel::Info => 'I',
            LogLevel::Warn => 'W',
            LogLevel::Error => 'E',
            LogLevel::Debug => 'D',
            LogLevel::Crit => 'C',
            LogLevel::Off => 'O',
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Crit => write!(f, "CRIT"),
            LogLevel::Off => write!(f, "OFF"),

        }
    }
}

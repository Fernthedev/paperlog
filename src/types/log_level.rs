use std::fmt::Display;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
    Crit,
    Off,
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

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
            LogLevel::Crit => "CRIT",
            LogLevel::Off => "OFF",
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `pad` rather than `write_str` so width/alignment specifiers
        // (`{:<5}`) still work.
        f.pad(self.as_str())
    }
}

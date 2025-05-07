use tracing::{debug, error, info, warn};

use crate::log_level::LogLevel;

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            // LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Crit => tracing::Level::ERROR,
            LogLevel::Off => tracing::Level::ERROR,
        }
    }
}

pub fn do_log(log: &super::LogData) -> crate::Result<()> {
    let message = log.format();
    match log.level {
        LogLevel::Info => info!("{message}"),
        LogLevel::Warn => warn!("{message}"),
        LogLevel::Error => error!("{message}"),
        LogLevel::Debug => debug!("{message}"),
        LogLevel::Crit => {
            error!("{message}");
        }
        LogLevel::Off => {
            // No logging
        }
    }

    Ok(())
}

use color_eyre::eyre::Result;
use tracing::{debug, error, event, info, warn};

use crate::log_level::LogLevel;

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            // LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::DEBUG => tracing::Level::DEBUG,
            LogLevel::INFO => tracing::Level::INFO,
            LogLevel::WARN => tracing::Level::WARN,
            LogLevel::ERROR => tracing::Level::ERROR,
        }
    }
}

pub fn do_log(log: &super::LogData) -> Result<()> {
    let message = log.format();
    match log.level {
        LogLevel::INFO => info!("{message}"),
        LogLevel::WARN => warn!("{message}"),
        LogLevel::ERROR => error!("{message}"),
        LogLevel::DEBUG => debug!("{message}"),
    }

    Ok(())
}

use tracing::{debug, error, info, warn};

use crate::types::log_level::LogLevel;

use crate::types::logger_chunk::LogDataChunk;

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

/// Re-emits the chunk as a `tracing` event. The chunk is passed as a
/// `Display` argument, so nothing is formatted (or allocated) unless a
/// subscriber actually records the event.
pub fn do_log(log: LogDataChunk<'_>) -> crate::Result<()> {
    match log.original.level {
        LogLevel::Info => info!("{log}"),
        LogLevel::Warn => warn!("{log}"),
        LogLevel::Error | LogLevel::Crit => error!("{log}"),
        LogLevel::Debug => debug!("{log}"),
        LogLevel::Off => {
            // No logging
        }
    }

    Ok(())
}

use crate::Result;

use super::log_data::LogData;

pub(crate) fn do_log(
    log: &LogData,
    logger_thread: &std::sync::RwLock<super::LoggerThread>,
) -> Result<()> {
    let sinks = &logger_thread.read().unwrap().sinks;

    for sink in sinks {
        sink(log)?;
    }

    Ok(())
}

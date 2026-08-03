use crate::{
    logger::{logger_chunk::LogDataChunk, LoggerThreadCtx},
    Result,
};
use parking_lot::RwLock;

pub(crate) fn do_log(log: LogDataChunk<'_>, logger_thread: &RwLock<LoggerThreadCtx>) -> Result<()> {
    let sinks = &logger_thread.read().sinks;

    for sink in sinks {
        sink(log)?;
    }

    Ok(())
}

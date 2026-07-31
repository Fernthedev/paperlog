use crate::{Result, logger::logger_chunk::LogDataChunk};
use parking_lot::RwLock;

pub(crate) fn do_log(
    log: LogDataChunk<'_>,
    logger_thread: &RwLock<crate::logger::logger_thread_ctx::LoggerThreadCtx>,
) -> Result<()> {
    let sinks = &logger_thread.read().sinks;

    for sink in sinks {
        sink(log)?;
    }

    Ok(())
}

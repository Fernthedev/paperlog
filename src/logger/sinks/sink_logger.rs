use crate::{
    logger::{logger_chunk::LogDataChunk, LogCallback},
    Result,
};

/// Takes an already-locked slice of sinks rather than the lock itself, so
/// callers can lock once and reuse it across a whole batch of logs instead
/// of re-acquiring the lock per log line.
pub(crate) fn do_log(log: LogDataChunk<'_>, sinks: &[Box<dyn LogCallback>]) -> Result<()> {
    for sink in sinks {
        sink(log)?;
    }

    Ok(())
}

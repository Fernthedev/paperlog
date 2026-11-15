use crate::{
    logger::{logger_chunk::LogDataChunk, LogCallback},
    LoggerError,
};

/// Takes an already-locked slice of sinks rather than the lock itself, so
/// callers can lock once and reuse it across a whole batch of logs instead
/// of re-acquiring the lock per log line.
///
/// Every sink is called even if an earlier one fails — one broken sink
/// shouldn't starve the others. Each error is handed to `on_error`.
pub(crate) fn do_log(
    log: LogDataChunk<'_>,
    sinks: &[Box<dyn LogCallback>],
    mut on_error: impl FnMut(LoggerError),
) {
    for sink in sinks {
        if let Err(e) = sink(log) {
            on_error(e);
        }
    }
}

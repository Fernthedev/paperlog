use bstr::ByteSlice;

use crate::{
    logger::logger_chunk::LogDataChunk,
    types::{file_logger_map::FileLoggers, LogData},
};

pub(crate) fn do_log(log: &LogData, file_loggers: &mut FileLoggers) -> std::io::Result<()> {
    let (global_file, context_file) =
        file_loggers.writers_for(log.tag.as_ref().map(|tag| tag.as_bstr()));

    log.write_to_io(global_file)?;

    if let Some(context_file) = context_file {
        log.write_compact_to_io(context_file)?;
    }

    Ok(())
}

/// Write a batch of logs to the file-backed outputs. The caller holds the
/// `FileLoggers` lock once for the whole batch.
pub(crate) fn do_log_batch(
    logs: &[LogDataChunk<'_>],
    file_loggers: &mut FileLoggers,
) -> std::io::Result<()> {
    for log in logs {
        let (global_file, context_file) =
            file_loggers.writers_for(log.original.tag.as_ref().map(|tag| tag.as_bstr()));

        log.write_to_io(global_file)?;

        if let Some(context_file) = context_file {
            log.write_compact_to_io(context_file)?;
        }
    }

    Ok(())
}

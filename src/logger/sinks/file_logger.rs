use bstr::ByteSlice;

use crate::{
    logger::logger_chunk::LogDataChunk,
    types::{file_logger_map::FileLoggers, LogData},
};

pub(crate) fn do_log(log: &LogData, file_loggers: &FileLoggers) -> std::io::Result<()> {
    log.write_to_io(&mut *file_loggers.get_global_file().lock())?;

    let context_file = log
        .tag
        .as_ref()
        .and_then(|tag| file_loggers.get_writer(tag.as_bstr()));

    if let Some(context_file) = context_file {
        log.write_compact_to_io(&mut *context_file.lock())?;
    }

    Ok(())
}

/// Write a batch of logs to the file-backed outputs while holding each file's
/// lock only once for the whole batch, rather than once per log line.
pub(crate) fn do_log_batch(
    logs: &[LogDataChunk<'_>],
    file_loggers: &FileLoggers,
) -> std::io::Result<()> {
    let mut global_file = file_loggers.get_global_file().lock();

    for log in logs {
        log.write_to_io(&mut *global_file)?;

        let context_file = log
            .original
            .tag
            .as_ref()
            .and_then(|tag| file_loggers.get_writer(tag.as_bstr()));

        if let Some(context_file) = context_file {
            log.write_compact_to_io(&mut *context_file.lock())?;
        }
    }

    Ok(())
}

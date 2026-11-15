use crate::{
    logger::logger_chunk::LogDataChunk,
    types::{
        file_logger_map::{FileLoggers, Writers},
        LogData,
    },
};

fn write_chunk(chunk: LogDataChunk<'_>, file_loggers: &mut FileLoggers) -> std::io::Result<()> {
    let Writers { global, context } = file_loggers.writers_for(chunk.original.tag.as_deref());

    chunk.write_to_io(global)?;

    if let Some(context) = context {
        chunk.write_compact_to_io(context)?;
    }

    Ok(())
}

pub(crate) fn do_log(log: &LogData, file_loggers: &mut FileLoggers) -> std::io::Result<()> {
    write_chunk(
        LogDataChunk {
            original: log,
            message: log.message.as_ref(),
        },
        file_loggers,
    )
}

/// Write a batch of logs to the file-backed outputs. The caller holds the
/// `FileLoggers` lock once for the whole batch.
pub(crate) fn do_log_batch(
    logs: &[LogDataChunk<'_>],
    file_loggers: &mut FileLoggers,
) -> std::io::Result<()> {
    for &log in logs {
        write_chunk(log, file_loggers)?;
    }

    Ok(())
}

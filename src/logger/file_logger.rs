use parking_lot::RwLock;

use crate::logger::logger_thread_ctx::LoggerThreadCtx;

pub(crate) fn do_log(
    log: &super::LogData,
    logger_thread_lock: &RwLock<LoggerThreadCtx>,
) -> std::io::Result<()> {
    let mut logger_thread = logger_thread_lock.write();

    let global_file = &mut logger_thread.global_file;
    log.write_to_io(global_file)?;

    let context_file = log
        .tag
        .as_ref()
        .and_then(|tag| logger_thread.context_map.get_mut(tag));

    if let Some(context_file) = context_file {
        log.write_compact_to_io(context_file)?;
    }

    Ok(())
}

/// Write a batch of logs to the file-backed outputs while holding the write lock only once.
pub(crate) fn do_log_batch(
    logs: &[super::LogData],
    logger_thread_lock: &RwLock<LoggerThreadCtx>,
) -> std::io::Result<()> {
    let mut logger_thread = logger_thread_lock.write();

    for log in logs {
        let global_file = &mut logger_thread.global_file;
        log.write_to_io(global_file)?;

        if let Some(tag) = &log.tag {
            if let Some(context_file) = logger_thread.context_map.get_mut(tag) {
                log.write_compact_to_io(context_file)?;
            }
        }
    }

    Ok(())
}

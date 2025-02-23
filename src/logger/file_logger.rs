use std::sync::RwLock;

use super::LoggerThread;

pub(crate) fn do_log(
    log: &super::LogData,
    logger_thread_lock: &RwLock<LoggerThread>,
) -> std::io::Result<()> {
    let mut logger_thread = logger_thread_lock.write().unwrap();

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

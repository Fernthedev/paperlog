use std::{
    io::Write,
    sync::{Arc, RwLock},
};

use super::LoggerThread;

pub(crate) fn do_log(
    log: &super::LogData,
    logger_thread_lock: Arc<RwLock<LoggerThread>>,
) -> std::io::Result<()> {
    let log_line = format!("{}\n", log.format());

    let mut logger_thread = logger_thread_lock.write().unwrap();

    let global_file = &mut logger_thread.global_file;
    global_file.write_all(log_line.as_bytes())?;

    if let Some(context_file) = log
        .tag
        .as_ref()
        .and_then(|tag| logger_thread.context_map.get_mut(tag))
    {
        context_file.write_all(log_line.as_bytes())?;
    }

    Ok(())
}

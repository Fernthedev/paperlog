//! A small `log` crate facade that forwards `log` records into the `paper2` logger.
//!
//! Usage:
//! - Initialize the `paper2` logger (eg. `paper2::init_logger(...)`).
//! - Call `paper2_log::init_with_max_level(log::LevelFilter::Info)` to install
//!   this facade as the global `log` implementation.

use log::{LevelFilter, Log, Metadata, Record};
use paper2_ffi::{
    paper2_LogLevel, paper2_LogLevel_Debug, paper2_LogLevel_Error, paper2_LogLevel_Info,
    paper2_LogLevel_Warn, paper2_get_inited, paper2_queue_log_ffi, paper2_wait_for_flush,
};
use std::ffi::CString;
use std::os::raw::c_int;
use std::ptr;

/// The paper2 logger that implements the `log::Log` trait.
pub struct Paper2Logger;

impl Paper2Logger {
    /// Initializes the global logger with the specified maximum log level.
    #[cfg(all(feature = "std", target_has_atomic = "ptr"))]
    pub fn init_with_max_level(level: LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_boxed_logger(Box::new(Paper2Logger))?;
        log::set_max_level(level);
        Ok(())
    }
}

fn map_level(level: log::Level) -> paper2_LogLevel {
    use log::Level;
    match level {
        Level::Error => paper2_LogLevel_Error,
        Level::Warn => paper2_LogLevel_Warn,
        Level::Info => paper2_LogLevel_Info,
        Level::Debug => paper2_LogLevel_Debug,
        Level::Trace => paper2_LogLevel_Debug,
    }
}

impl Log for Paper2Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // If the paper2 logger is initialized (via FFI), forward via the FFI queue.
        unsafe {
            if !paper2_get_inited() {
                return;
            }
            let level = map_level(record.level());
            let tag_opt = record.module_path();
            let tag_c = tag_opt.and_then(|s| CString::new(s).ok());
            let message_c = CString::new(record.args().to_string()).ok();
            let file_c = record.file().and_then(|s| CString::new(s).ok());
            let function_c = record.module_path().and_then(|s| CString::new(s).ok());

            let tag_ptr = tag_c.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null());
            let msg_ptr = message_c
                .as_ref()
                .map(|s| s.as_ptr())
                .unwrap_or(ptr::null());
            let file_ptr = file_c.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null());
            let func_ptr = function_c
                .as_ref()
                .map(|s| s.as_ptr())
                .unwrap_or(ptr::null());

            let line = record.line().unwrap_or(0) as c_int;

            // Best-effort: ignore return value
            let _ = paper2_queue_log_ffi(level, tag_ptr, msg_ptr, file_ptr, line, 0, func_ptr);
        }
    }

    fn flush(&self) {
        unsafe {
            if paper2_get_inited() {
                let _ = paper2_wait_for_flush();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use paper2_ffi::paper2_init_logger_ffi;

    use super::*;
    // Basic smoke test: using the facade directly to send a single record via FFI.
    #[test]
    fn install_and_log() {
        // initialize a paper2 logger (global temporary path)
        let tmp = std::env::temp_dir().join(format!("paperlog_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let log_path = tmp.join("paper2_log_smoke.log");

        // Initialize via the C FFI. Pass a null config to use defaults and the
        // path as a C string.
        let path_c = CString::new(log_path.to_string_lossy().into_owned()).unwrap();
        unsafe {
            let _ = paper2_init_logger_ffi(ptr::null(), path_c.as_ptr());
        }

        let _ = Paper2Logger::init_with_max_level(LevelFilter::Info);

        // Build a `Record` and call the facade directly so we don't depend on
        // the availability of `log::set_boxed_logger` in the test environment.
        let record = Record::builder()
            .args(format_args!("paper2_log facade test message"))
            .level(log::Level::Info)
            .module_path_static(Some("paper2_log_test"))
            .file_static(Some("test.rs"))
            .line(Some(1))
            .build();

        Paper2Logger.log(&record);

        // Poll for a short while for the message to be flushed to disk.
        let mut found = false;
        for _ in 0..200 {
            if let Ok(contents) = std::fs::read_to_string(&log_path)
                && contents.contains("paper2_log facade test message")
            {
                found = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert!(found, "expected log message in {}", log_path.display());
        let _ = std::fs::remove_file(&log_path);
    }
}

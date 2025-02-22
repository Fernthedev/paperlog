use crate::get_logger;
use crate::init_logger;
use crate::log_level::LogLevel;
use crate::logger::LogData;
use crate::logger::LoggerConfig;
use std::ffi::c_uint;
use std::ffi::{c_uchar, c_ulonglong, CStr};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone)]
pub struct LoggerConfigFfi {
    pub max_string_len: c_ulonglong,
    pub log_max_buffer_count: c_ulonglong,
    pub line_end: c_uchar,
    pub context_log_path: *const c_char,
}

#[no_mangle]
pub unsafe extern "C" fn paper2_init_logger_ffi(
    config: *const LoggerConfigFfi,
    path: *const c_char,
) -> bool {
    if path.is_null() {
        return false;
    }

    let c_str = unsafe { CStr::from_ptr(path) };
    let path_str = match c_str.to_str() {
        Ok(str) => str,
        Err(_) => return false,
    };

    let path_buf = PathBuf::from(path_str);

    let converted_config: LoggerConfig = unsafe {
        config
            .as_ref()
            .map(|config| -> LoggerConfig { config.clone().into() })
            .unwrap_or_default()
    };

    init_logger(converted_config, path_buf).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn paper2_register_context_id(tag: *const c_char) {
    if tag.is_null() {
        return;
    }

    let Some(logger) = get_logger() else {
        return;
    };

    let tag = unsafe { CStr::from_ptr(tag).to_string_lossy() };

    let result = logger.write().unwrap().add_context(&tag);

    if let Err(report) = result {
        logger.read().unwrap().queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: format!("Error creating context {tag}:\n{}", report),
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
    }
}

#[no_mangle]
pub unsafe extern "C" fn paper2_unregister_context_id(tag: *const c_char) {
    if tag.is_null() {
        return;
    }

    let Some(logger) = get_logger() else {
        return;
    };

    let tag = unsafe { CStr::from_ptr(tag).to_string_lossy() };

    logger.write().unwrap().remove_context(&tag);
}

#[no_mangle]
pub unsafe extern "C" fn paper2_queue_log_ffi(
    level: LogLevel,
    tag: *const c_char,
    message: *const c_char,
    file: *const c_char,
    line: c_int,
    column: c_int,
    function_name: *const c_char,
) -> bool {
    if message.is_null() || file.is_null() {
        return false;
    }

    let Some(logger) = get_logger() else {
        return false;
    };

    let tag = unsafe {
        tag.as_ref()
            .map(|c_str| CStr::from_ptr(c_str))
            .map(|c| c.to_string_lossy().into_owned())
    };

    let message = unsafe { CStr::from_ptr(message).to_string_lossy().into_owned() };
    let file = unsafe { CStr::from_ptr(file).to_string_lossy().into_owned() };

    let log_data = LogData {
        level,
        tag,
        message,
        file,
        line: line as u32,
        column: column as u32,
        function_name: unsafe {
            function_name
                .as_ref()
                .map(|c_str| CStr::from_ptr(c_str))
                .map(|c| c.to_string_lossy().into_owned())
        },
        ..Default::default()
    };

    logger.read().unwrap().queue_log(log_data);

    true
}

#[no_mangle]
pub unsafe extern "C" fn paper2_wait_for_flush() -> bool {
    let Some(logger) = get_logger() else {
        return false;
    };

    logger.read().unwrap().wait_for_flush();

    true
}

#[no_mangle]
pub unsafe extern "C" fn paper2_get_log_directory() -> *const c_char {
    let Some(logger) = get_logger() else {
        return std::ptr::null();
    };

    let log_directory = logger
        .read()
        .unwrap()
        .config
        .context_log_path
        .to_string_lossy()
        .into_owned();

    let c_str = std::ffi::CString::new(log_directory).unwrap();
    c_str.into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn paper2_get_inited() -> bool {
    let Some(logger) = get_logger() else {
        return false;
    };

    let is_inited = logger
        .read()
        .unwrap()
        .is_inited()
        .load(std::sync::atomic::Ordering::SeqCst);

    is_inited
}

#[no_mangle]
pub unsafe extern "C" fn paper2_wait_flush_timeout(timeout_ms: c_uint) -> bool {
    let Some(logger) = get_logger() else {
        return false;
    };

    logger
        .read()
        .unwrap()
        .wait_for_flush_timeout(std::time::Duration::from_millis(timeout_ms as u64));

    true
}

impl From<LoggerConfigFfi> for LoggerConfig {
    fn from(ffi: LoggerConfigFfi) -> Self {
        Self {
            max_string_len: ffi.max_string_len as usize,
            log_max_buffer_count: ffi.log_max_buffer_count as usize,
            line_end: ffi.line_end as char,
            context_log_path: unsafe {
                CStr::from_ptr(ffi.context_log_path)
                    .to_string_lossy()
                    .into_owned()
                    .into()
            },
        }
    }
}

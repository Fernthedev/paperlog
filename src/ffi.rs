use crate::ffi::c_str_helper::StringRef;
use crate::get_logger;
use crate::init_logger;
use crate::log_level::LogLevel;
use crate::logger::LogData;
use crate::logger::LoggerConfig;
use crate::Result;
use std::ffi::c_uint;
use std::ffi::{c_uchar, c_ulonglong, CStr};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

mod c_str_helper;

/// Extern "C" compatible log callback type.
/// The callback receives a pointer to LogData and a user-provided context pointer.
/// Returns 0 for success, nonzero for error.
pub type LogCallbackC =
    unsafe extern "C" fn(log_data: *const LogDataC, user_data: *mut std::ffi::c_void);

#[repr(C)]
pub struct LogDataC<'a> {
    pub level: LogLevel,
    pub tag: StringRef<'a>,
    pub message: StringRef<'a>,
    pub timestamp: i64, // Unix timestamp in seconds

    pub file: StringRef<'a>,
    pub line: u32,
    pub column: u32,
    pub function_name: StringRef<'a>,
}

#[repr(C)]
#[derive(Clone)]
/// FFI-safe configuration for the logger.
///
/// # Safety
/// - `context_log_path` must be a valid, null-terminated C string.
/// - All fields must be properly initialized before passing to FFI functions.
pub struct LoggerConfigFfi {
    pub max_string_len: c_ulonglong,
    pub log_max_buffer_count: c_ulonglong,
    pub line_end: c_uchar,
    pub context_log_path: *const c_char,
}

#[no_mangle]
/// Initializes the logger from FFI.
///
/// # Safety
/// - `config` and `path` must be valid pointers to initialized data.
/// - `path` must be a valid, null-terminated C string.
/// - This function should only be called once during initialization.
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
/// Registers a new logging context by ID.
///
/// # Safety
/// - `tag` must be a valid, null-terminated C string.
pub unsafe extern "C" fn paper2_register_context_id(tag: *const c_char) {
    if tag.is_null() {
        return;
    }

    let Some(logger) = get_logger() else {
        return;
    };

    let tag = unsafe { CStr::from_ptr(tag).to_string_lossy() };

    let result = logger.write().add_context(&tag);

    if let Err(report) = result {
        logger.read().queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: format!("Error creating context {tag}:\n{report}"),
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
    }
}

#[no_mangle]
/// Unregisters a logging context by ID.
///
/// # Safety
/// - `tag` must be a valid, null-terminated C string.
pub unsafe extern "C" fn paper2_unregister_context_id(tag: *const c_char) {
    if tag.is_null() {
        return;
    }

    let Some(logger) = get_logger() else {
        return;
    };

    let tag = unsafe { CStr::from_ptr(tag).to_string_lossy() };

    logger.write().remove_context(&tag);
}

#[no_mangle]
/// Queues a log entry from FFI.
///
/// # Safety
/// - All pointer arguments must be valid, null-terminated C strings.
/// - `level` must be a valid `LogLevel`.
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

    logger.read().queue_log(log_data);

    true
}

#[no_mangle]
/// Waits for all logs to be flushed.
///
/// # Safety
/// - No arguments. Safe to call if logger is initialized.
pub unsafe extern "C" fn paper2_wait_for_flush() -> bool {
    let Some(logger) = get_logger() else {
        return false;
    };

    logger.read().wait_for_flush();

    true
}

#[no_mangle]
/// Gets the log directory as a C string.
///
/// # Safety
/// - Returns a pointer to a heap-allocated C string. Caller must eventually free it.
/// - Safe to call if logger is initialized.
pub unsafe extern "C" fn paper2_get_log_directory() -> *const c_char {
    // If the `file` feature is disabled, there is no context log path.
    #[cfg(not(feature = "file"))]
    {
        return std::ptr::null();
    }

    #[cfg(feature = "file")]
    {
        let Some(logger) = get_logger() else {
            return std::ptr::null();
        };

        let log_directory = logger
            .read()
            .config
            .context_log_path
            .to_string_lossy()
            .into_owned();

        let c_str = std::ffi::CString::new(log_directory).unwrap();
        c_str.into_raw()
    }
}

#[no_mangle]
/// Returns whether the logger is initialized.
///
/// # Safety
/// - No arguments. Safe to call at any time.
pub unsafe extern "C" fn paper2_get_inited() -> bool {
    let Some(logger) = get_logger() else {
        return false;
    };

    let is_inited = logger
        .read()
        .is_inited()
        .load(std::sync::atomic::Ordering::SeqCst);

    is_inited
}

#[no_mangle]
/// Waits for all logs to be flushed, with a timeout in milliseconds.
///
/// # Safety
/// - Safe to call if logger is initialized.
pub unsafe extern "C" fn paper2_wait_flush_timeout(timeout_ms: c_uint) -> bool {
    let Some(logger) = get_logger() else {
        return false;
    };

    logger
        .read()
        .wait_for_flush_timeout(std::time::Duration::from_millis(timeout_ms as u64));

    true
}

/// Shuts down the logger, flushing all logs.
/// # Safety
/// - Safe to call if logger is initialized.
#[no_mangle]
pub unsafe extern "C" fn paper2_add_log_sink(
    callback: LogCallbackC,
    user_data: *mut std::ffi::c_void,
) -> bool {
    let Some(logger) = get_logger() else {
        return false;
    };

    // Wrap the user_data pointer in an Arc<AtomicPtr<c_void>> to make it Send + Sync
    let user_data_ptr = Arc::new(AtomicPtr::new(user_data));

    logger
        .write()
        .add_sink(move |data: &LogData| -> Result<()> {
            let c_data: LogDataC = data.into();
            let user_data = user_data_ptr.load(Ordering::SeqCst);
            callback(&c_data, user_data);
            Ok(())
        });

    true
}

/// Frees a C string allocated by `paper2_get_log_directory`.
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn paper2_free_c_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = std::ffi::CString::from_raw(s);
    }
}

/// Converts FFI logger config to internal config.
///
/// # Safety
/// - `context_log_path` must be a valid, null-terminated C string.
impl From<LoggerConfigFfi> for LoggerConfig {
    fn from(ffi: LoggerConfigFfi) -> Self {
        #[cfg(feature = "file")]
        {
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

        #[cfg(not(feature = "file"))]
        {
            Self {
                max_string_len: ffi.max_string_len as usize,
                log_max_buffer_count: ffi.log_max_buffer_count as usize,
                line_end: ffi.line_end as char,
            }
        }
    }
}

/// Converts internal LogData to FFI-compatible LogDataC.
/// # Safety
/// - All string fields are converted to C strings. Caller must ensure proper memory management.
impl<'a> From<&'a LogData> for LogDataC<'a> {
    fn from(data: &'a LogData) -> Self {
        Self {
            level: data.level,
            tag: data.tag.as_deref().into(),
            message: data.message.as_str().into(),
            timestamp: data.timestamp.timestamp(),

            file: data.file.as_str().into(),
            line: data.line,
            column: data.column,
            function_name: data.function_name.as_deref().into(),
        }
    }
}

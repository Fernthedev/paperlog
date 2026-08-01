use bstr::{ByteSlice, ByteVec};

use crate::ffi::c_str_helper::StringRef;
use crate::get_logger;
use crate::init_logger;
use crate::types::config::LoggerConfig;
use crate::types::log_level::LogLevel;
use crate::types::logger_chunk::LogDataChunk;
use crate::types::LogData;
use crate::types::PaperStr;
use crate::types::PaperString;
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

    pub file: StringRef<'static>,
    pub line: u32,
    pub column: u32,
    pub function_name: StringRef<'static>,
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
    // `to_path_lossy()` is zero-copy on unix (raw bytes reinterpreted as
    // `OsStr`), so a non-UTF-8 path still works instead of failing init.
    let path_buf: PathBuf = c_str.to_bytes().to_path_lossy().into_owned();

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

    let tag = unsafe { PaperStr::new(CStr::from_ptr(tag).to_bytes()) };

    let result = logger.write().add_context(tag);

    if let Err(report) = result {
        logger.read().queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: format!("Error creating context {tag}:\n{report}").into(),
            file: file!().into(),
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

    let tag = unsafe { PaperStr::new(CStr::from_ptr(tag).to_bytes()) };

    logger.write().remove_context(tag);
}

#[no_mangle]
/// Queues a log entry from FFI.
///
/// # Safety
/// - `tag`/`function_name` may be null (meaning "absent"); if non-null, and
///   `message`/`file` always, must be valid, null-terminated C strings.
/// - `file` and `function_name` are stored as `&'static` references without
///   copying — the pointed-to memory must remain valid for the rest of the
///   process's lifetime. This holds for the common case (`__FILE__`/
///   `__builtin_FUNCTION()`/`std::source_location`, all backed by static
///   storage) but NOT for a transient/stack/heap buffer built per call; do
///   not pass one of those here. `tag` and `message` are copied, so they
///   have no such requirement.
/// - `level` must be a valid `LogLevel`.
///
/// Prefer [`paper2_queue_log_bytes_ffi`] when the caller already knows each
/// string's length (e.g. `std::string_view::size()`) — it avoids a `strlen`
/// scan per field here, and doesn't require `tag`/`function_name` to be
/// null-terminated at all.
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
            .map(|c| PaperString::from(c.to_bytes()))
    };

    let message: &'static CStr = unsafe { CStr::from_ptr(message) };
    let file: &'static CStr = unsafe { CStr::from_ptr(file) };

    let log_data = LogData {
        level,
        tag,
        message: PaperString::from(message.to_bytes()),
        file: PaperStr::new(file.to_bytes()),

        line: line as u32,
        column: column as u32,
        function_name: unsafe {
            function_name
                .as_ref()
                .map(|c_str| CStr::from_ptr(c_str))
                .map(|c: &'static CStr| PaperStr::new(c.to_bytes()))
        },
        ..Default::default()
    };

    logger.read().queue_log(log_data);

    true
}

#[no_mangle]
/// Queues a log entry from FFI using explicit `(pointer, length)` pairs
/// instead of null-terminated C strings.
///
/// Prefer this over [`paper2_queue_log_ffi`] whenever the caller already has
/// a length on hand (`std::string_view`/`std::string` both do, for free).
/// Benefits over the C-string version:
/// - No `strlen` scan for any of the four fields.
/// - `tag`/`function_name` don't need to be null-terminated at all — a
///   `std::string_view` that's a substring of a larger buffer is safe to
///   pass directly, which it is *not* for [`paper2_queue_log_ffi`].
/// - Embedded NUL bytes in the middle of a string are preserved rather than
///   silently truncating it, consistent with this crate's non-UTF-8-safe,
///   arbitrary-byte-string design.
///
/// # Safety
/// - `message_ptr`/`file_ptr` must be non-null and valid for reads of
///   `message_len`/`file_len` bytes.
/// - `tag_ptr`/`function_name_ptr` may be null (meaning "absent"); if
///   non-null, must be valid for reads of `tag_len`/`function_name_len`
///   bytes.
/// - `file` and `function_name` are stored as `&'static` references without
///   copying — see the equivalent note on [`paper2_queue_log_ffi`]; the
///   same static-storage requirement applies here.
/// - `level` must be a valid `LogLevel`.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn paper2_queue_log_bytes_ffi(
    level: LogLevel,
    tag_ptr: *const u8,
    tag_len: usize,
    message_ptr: *const u8,
    message_len: usize,
    file_ptr: *const u8,
    file_len: usize,
    line: c_int,
    column: c_int,
    function_name_ptr: *const u8,
    function_name_len: usize,
) -> bool {
    if message_ptr.is_null() || file_ptr.is_null() {
        return false;
    }

    let Some(logger) = get_logger() else {
        return false;
    };

    let tag = (!tag_ptr.is_null())
        .then(|| unsafe { std::slice::from_raw_parts(tag_ptr, tag_len) })
        .map(PaperString::from);

    let message: &'static [u8] = unsafe { std::slice::from_raw_parts(message_ptr, message_len) };
    let file: &'static [u8] = unsafe { std::slice::from_raw_parts(file_ptr, file_len) };

    let function_name = (!function_name_ptr.is_null())
        .then(|| unsafe { std::slice::from_raw_parts(function_name_ptr, function_name_len) })
        .map(PaperStr::new);

    let log_data = LogData {
        level,
        tag,
        message: PaperString::from(message),
        file: PaperStr::new(file),

        line: line as u32,
        column: column as u32,
        function_name,
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

        // `Vec::from_path_lossy` is zero-copy on unix, so this doesn't
        // require the configured path to be valid UTF-8.
        let log_directory =
            Vec::from_path_lossy(&logger.read().config.context_log_path).into_owned();

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
    // `LOGGER` is only ever populated (via `init_logger`) with an instance
    // that has already gone through `LoggerThreadCtx::init()`, so its
    // presence alone tells us the logger is initialized without needing to
    // take the `RwLock` and load the atomic flag too. This is called on
    // every log line from the `log`-crate facade, so keeping it to a single
    // `OnceLock` load (no lock, no second atomic) is worth it.
    get_logger().is_some()
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
        .add_sink(move |chunk: LogDataChunk<'_>| -> Result<()> {
            let c_data: LogDataC = chunk.into();
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
                // `to_path_lossy()` is zero-copy on unix, so a non-UTF-8
                // configured path doesn't get rejected or mangled here.
                context_log_path: unsafe {
                    CStr::from_ptr(ffi.context_log_path)
                        .to_bytes()
                        .to_path_lossy()
                        .into_owned()
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

/// Converts a log chunk to an FFI-compatible `LogDataC`.
///
/// Fields are exposed as raw `(ptr, len)` byte views (see `StringRef`)
/// rather than validated UTF-8 strings, since the underlying bytes
/// ultimately come from a caller-supplied C string that isn't guaranteed to
/// be valid UTF-8.
impl<'a> From<LogDataChunk<'a>> for LogDataC<'a> {
    fn from(chunk: LogDataChunk<'a>) -> Self {
        let data = chunk.original;
        Self {
            level: data.level,
            tag: data.tag.as_ref().map(|s| s.as_bstr()).into(),
            message: chunk.message.into(),
            timestamp: data.timestamp.timestamp(),

            file: data.file.into(),
            line: data.line,
            column: data.column,
            function_name: data.function_name.into(),
        }
    }
}

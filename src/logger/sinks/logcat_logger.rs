// https://github.com/raftario/paranoid-android/blob/main/src/writer.rs

use crate::types::PaperStr;
use crate::types::log_level::LogLevel;
use crate::types::logger_chunk::LogDataChunk;
use crate::Result;

use std::ffi::{CStr, CString};
use std::io::Write as _;

// // assert tracing is not enabled
// #[cfg(feature = "tracing")]
// compile_error!("The 'tracing' feature must not be enabled to use this logger.");

use ndk_sys::{android_LogPriority, log_id};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Verbose = android_LogPriority::ANDROID_LOG_VERBOSE.0,
    Debug = android_LogPriority::ANDROID_LOG_DEBUG.0,
    Info = android_LogPriority::ANDROID_LOG_INFO.0,
    Warn = android_LogPriority::ANDROID_LOG_WARN.0,
    Error = android_LogPriority::ANDROID_LOG_ERROR.0,
    Fatal = android_LogPriority::ANDROID_LOG_FATAL.0,
    Silent = android_LogPriority::ANDROID_LOG_SILENT.0,
}

/// An [Android log buffer](https://developer.android.com/ndk/reference/group/logging#log_id).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buffer {
    /// Let the logging function choose the best log target.
    Default = log_id::LOG_ID_DEFAULT.0,

    /// The main log buffer.
    ///
    /// This is the only log buffer available to apps.
    Main = log_id::LOG_ID_MAIN.0,

    /// The crash log buffer.
    Crash = log_id::LOG_ID_CRASH.0,
    /// The statistics log buffer.
    Stats = log_id::LOG_ID_STATS.0,
    /// The event log buffer.
    Events = log_id::LOG_ID_EVENTS.0,
    /// The security log buffer.
    Security = log_id::LOG_ID_SECURITY.0,
    /// The system log buffer.
    System = log_id::LOG_ID_SYSTEM.0,
    /// The kernel log buffer.
    Kernel = log_id::LOG_ID_KERNEL.0,
    /// The radio log buffer.
    Radio = log_id::LOG_ID_RADIO.0,
}

impl From<LogLevel> for Priority {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Crit => Priority::Fatal,
            LogLevel::Off => Priority::Silent,
            LogLevel::Info => Priority::Info,
            LogLevel::Warn => Priority::Warn,
            LogLevel::Error => Priority::Error,
            LogLevel::Debug => Priority::Debug,
        }
    }
}

thread_local! {
    static CSTRING_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(Vec::with_capacity(8092));
}

/// Appends `s` plus a nul terminator to `buf`, and returns a closure that
/// later resolves to a `&CStr` view of just the segment written here, given
/// `buf`'s eventual final contents.
///
/// This can't hand back the `&CStr` directly: a later append (or the
/// `write!` for the message) can still grow/reallocate `buf`, which would
/// dangle an already-borrowed slice, and the borrow checker wouldn't allow
/// reusing `buf` for the next append while an earlier live `&CStr` from it
/// exists anyway. The returned closure instead captures only two plain
/// `usize` offsets — not a reference into `buf` — so `buf`'s mutable borrow
/// ends the moment this function returns. Any number of these can be
/// created across several appends and then all resolved together, once,
/// from one shared immutable borrow after every append is done.
fn append_string_to_buf(buf: &mut Vec<u8>, s: &PaperStr) -> impl Fn(&[u8]) -> &CStr {
    let start = buf.len();
    buf.extend_from_slice(s);
    buf.push(0);
    let end = buf.len();
    move |buf: &[u8]| unsafe { CStr::from_bytes_with_nul_unchecked(&buf[start..end]) }
}

pub(crate) fn do_log(log: LogDataChunk<'_>) -> Result<()> {
    let data = log.original;
    let priority: Priority = data.level.into();

    CSTRING_BUFFER.with_borrow_mut(|buffer| unsafe {
        let tag = log.tag();

        // Reserve a rough estimate up front; `write!`/`extend_from_slice`
        // below will grow the buffer further if needed. The buffer is
        // thread-local and reused across calls, so this only costs a
        // reallocation on the first few (growing) calls.
        let estimate = tag.len() + data.file.len() + log.message.len() + 32;
        buffer.reserve(estimate.saturating_sub(buffer.capacity()));

        // tag, and (only when needed) file, are written directly into the
        // reusable buffer (no intermediate `String` allocation) and
        // null-terminated in place so they can be handed to the NDK as C
        // strings.
        let tag_view = append_string_to_buf(buffer, tag);

        // The plain `__android_log_write` path below only takes a tag and a
        // message; `file` as its own C string is only read by the
        // `android-api-30` log-message struct, so skip that copy otherwise.
        #[cfg(feature = "android-api-30")]
        let file_view = append_string_to_buf(buffer, data.file);

        let message_start = buffer.len();
        write!(
            buffer,
            "[{file}:{line}:{column} @ {function_name}] {message}\0",
            file = data.file,
            line = data.line,
            column = data.column,
            function_name = data.function_name_or_default(),
            message = log.message,
        )
        .expect("writing to an in-memory buffer cannot fail");

        let tag = tag_view(buffer);
        let message = CStr::from_bytes_with_nul_unchecked(&buffer[message_start..]);

        #[cfg(feature = "android-api-30")]
        {
            use ndk_sys::{
                __android_log_is_loggable, __android_log_message, __android_log_write_log_message,
            };

            let file = file_view(buffer);

            if __android_log_is_loggable(priority as i32, tag.as_ptr(), priority as i32) == 0 {
                return;
            }

            let mut message = __android_log_message {
                struct_size: size_of::<__android_log_message>(),
                buffer_id: Buffer::Main as i32,
                priority: priority as i32,
                tag: tag.as_ptr(),
                file: file.as_ptr(),
                line: data.line,
                message: message.as_ptr(),
            };

            __android_log_write_log_message(&mut message);
        }

        #[cfg(not(feature = "android-api-30"))]
        {
            use ndk_sys::__android_log_write;

            __android_log_write(priority as i32, tag.as_ptr(), message.as_ptr());
        }

        buffer.clear();
    });
    // tag, priority, and time are provided by android's logcat

    Ok(())

    // #[cfg(not(feature = "api-30"))]
    // {
    //     use ndk_sys::__android_log_buf_write;

    //     for message in messages {
    //         unsafe { __android_log_buf_write(buffer, priority, tag, message) };
    //     }
    // }
}

pub fn log_error(message_str: String) {
    use ndk_sys::{__android_log_write, android_LogPriority};

    let priority = android_LogPriority::ANDROID_LOG_ERROR.0;
    let tag = CString::from(c"Paper2");
    let msg = match CString::new(message_str) {
        Ok(s) => s,
        Err(_e) => {
            return;
        }
    };

    unsafe { __android_log_write(priority as i32, tag.as_ptr(), msg.as_ptr()) };
}

pub fn log_info(message_str: String) {
    use ndk_sys::{__android_log_write, android_LogPriority};

    let priority = android_LogPriority::ANDROID_LOG_INFO.0;
    let tag = CString::from(c"Paper2");
    let msg = match CString::new(message_str) {
        Ok(s) => s,
        Err(_e) => {
            return;
        }
    };

    unsafe { __android_log_write(priority as i32, tag.as_ptr(), msg.as_ptr()) };
}

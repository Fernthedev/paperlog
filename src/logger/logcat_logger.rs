// https://github.com/raftario/paranoid-android/blob/main/src/writer.rs

use crate::{log_level::LogLevel, Result};

use std::ffi::{CStr, CString};

// assert tracing is not enabled
#[cfg(feature = "tracing")]
compile_error!("The 'tracing' feature must be enabled to use this logger.");

use ndk_sys::__android_log_is_loggable;
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

pub(crate) fn do_log(log: &super::log_data::LogData) -> Result<()> {
    let message_str = format!(
        "[{}:{}:{} @ {}] {}",
        log.file,
        log.line,
        log.column,
        log.function_name.as_deref().unwrap_or(""),
        log.message.clone()
    );

    let priority: Priority = log.level.clone().into();
    CSTRING_BUFFER.with(|buf| unsafe {
        let mut buffer = buf.borrow_mut();

        let tag = log.tag.as_deref().unwrap_or("default");

        let sizes = tag.len() + log.file.len() + message_str.len() + 3;
        if buffer.capacity() < sizes {
            let capacity = buffer.capacity();
            buffer.reserve(sizes - capacity);
        }

        // tag, file, and message are null-terminated
        buffer.extend(tag.bytes().chain(std::iter::once(b'\0')));
        buffer.extend(log.file.bytes().chain(std::iter::once(b'\0')));
        buffer.extend(message_str.bytes().chain(std::iter::once(b'\0')));

        let tag_len = tag.len() + 1;
        let file_len = tag_len + log.file.len() + 1;
        let message_len = message_str.len() + 1;

        let tag = CStr::from_bytes_with_nul_unchecked(&buffer[0..tag_len]);
        let file = CStr::from_bytes_with_nul_unchecked(&buffer[tag_len..file_len]);
        let message = CStr::from_bytes_with_nul_unchecked(&buffer[file_len..message_len]);

        #[cfg(feature = "android-api-30")]
        {
            use ndk_sys::{__android_log_message, __android_log_write_log_message};

            if unsafe { __android_log_is_loggable(priority as i32, tag.as_ptr(), priority as i32) }
                == 0
            {
                return Ok(());
            }

            let mut message = __android_log_message {
                struct_size: size_of::<__android_log_message>(),
                buffer_id: Buffer::Main as i32,
                priority: priority as i32,
                tag: tag.as_ptr(),
                file: file.as_ptr(),
                line: log.line,
                message: message.as_ptr(),
            };

            unsafe { __android_log_write_log_message(&mut message) };
        }

        #[cfg(not(feature = "android-api-30"))]
        {
            use ndk_sys::__android_log_write;

            unsafe { __android_log_write(priority as i32, tag.as_ptr(), message.as_ptr()) };
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

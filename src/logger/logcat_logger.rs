// https://github.com/raftario/paranoid-android/blob/main/src/writer.rs

use crate::{log_level::LogLevel, Result};

use std::ffi::CString;

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

pub(crate) fn do_log(log: &super::log_data::LogData) -> Result<()> {
    // #[cfg(feature = "api-30")]

    let message_str = format!(
        "{}:{}:{} @ {} {}",
        // really ugly way of getting last 50 chars
        log.file,
        log.line,
        log.column,
        log.function_name.as_deref().unwrap_or(""),
        log.message.clone()
    );

    let priority: Priority = log.level.clone().into();
    let tag = CString::new(log.tag.as_deref().unwrap_or("default"))?;
    let file = CString::new(log.file.to_string())?;
    let message = CString::new(message_str)?;
    // tag, priority, and time are provided by android's logcat

    if unsafe { __android_log_is_loggable(priority as i32, tag.as_ptr(), priority as i32) } == 0 {
        return Ok(());
    }

    #[cfg(feature = "android-api-30")]
    {
        use ndk_sys::{__android_log_message, __android_log_write_log_message};

        let mut message = __android_log_message {
            struct_size: size_of::<__android_log_message>(),
            buffer_id: Buffer::Default as i32,
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
        use ndk_sys::__android_log_buf_write;

        unsafe { __android_log_buf_write(Buffer::Default as i32, priority as i32, tag.as_ptr(), message.as_ptr()) };
    }

    Ok(())

    // #[cfg(not(feature = "api-30"))]
    // {
    //     use ndk_sys::__android_log_buf_write;

    //     for message in messages {
    //         unsafe { __android_log_buf_write(buffer, priority, tag, message) };
    //     }
    // }
}

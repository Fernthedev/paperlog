use std::{ffi::CStr, path::PathBuf};
use std::ffi::CString;
use ctor::ctor;
use paper2::LoggerConfig;

use mimalloc::MiMalloc;

// using mimalloc for android
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[ctor]
fn dlopen_initialize() {
    log_info(String::from("DLOpen initializing"));

    let id = unsafe { CStr::from_ptr(scotland2_rs::scotland2_raw::modloader_get_application_id()) }
        .to_string_lossy();
    let path = PathBuf::from(format!("/sdcard/ModData/{id}/logs2",));
    let config = LoggerConfig {
        context_log_path: path.clone(),
        ..LoggerConfig::default()
    };

    let file = path.join("Paperlog.log");
    if let Err(e) = paper2::init_logger(config, file) {
        log_info(
            String::from(
                format!("Error occurred in logging thread: {}", e)
            )
        );
        panic!("Error occurred in logging thread: {}", e);
    }
}

fn log_error(message_str: String) {
    use ndk_sys::__android_log_write;
    use ndk_sys::{android_LogPriority, log_id};

    let priority = android_LogPriority::ANDROID_LOG_ERROR.0;
    let tag = CString::from(c"E");
    let msg = match CString::new(message_str) {
        Ok(s) => s,
        Err(e) => {
            return;
        }
    };

    unsafe { __android_log_write(priority as i32, tag.as_ptr(), msg.as_ptr()) };
}

fn log_info(message_str: String)  {
    use ndk_sys::__android_log_buf_write;
    use ndk_sys::__android_log_write;
    use ndk_sys::{android_LogPriority, log_id};

    let priority = android_LogPriority::ANDROID_LOG_INFO.0;
    let tag = CString::from(c"I");
    let msg = match CString::new(message_str) {
        Ok(s) => s,
        Err(e) => {
            return;
        }
    };

    unsafe { __android_log_write(priority as i32, tag.as_ptr(), msg.as_ptr()) };
}

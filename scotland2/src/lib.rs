use ctor::ctor;
use paper2::LoggerConfig;
use std::backtrace::Backtrace;
use std::panic::PanicHookInfo;
use std::{ffi::CStr, ffi::CString, path::PathBuf};

use mimalloc::MiMalloc;

// using mimalloc for android
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[ctor]
fn dlopen_initialize() {
    log_info(String::from("DLOpen initializing"));
    std::panic::set_hook(panic_hook(true));

    let id = unsafe { CStr::from_ptr(scotland2_rs::scotland2_raw::modloader_get_application_id()) }
        .to_string_lossy();
    let path = PathBuf::from(format!("/sdcard/ModData/{id}/logs2",));
    let config = LoggerConfig {
        context_log_path: path.clone(),
        ..LoggerConfig::default()
    };

    let file = path.join("Paperlog.log");
    if let Err(e) = paper2::init_logger(config, file) {
        log_info(String::from(format!(
            "Error occurred in logging thread: {}",
            e
        )));
        panic!("Error occurred in logging thread: {}", e);
    }
}

fn log_error(message_str: String) {
    use ndk_sys::{__android_log_write, android_LogPriority};

    let priority = android_LogPriority::ANDROID_LOG_ERROR.0;
    let tag = CString::from(c"E");
    let msg = match CString::new(message_str) {
        Ok(s) => s,
        Err(_e) => {
            return;
        }
    };

    unsafe { __android_log_write(priority as i32, tag.as_ptr(), msg.as_ptr()) };
}

fn log_info(message_str: String) {
    use ndk_sys::{__android_log_write, android_LogPriority};

    let priority = android_LogPriority::ANDROID_LOG_INFO.0;
    let tag = CString::from(c"I");
    let msg = match CString::new(message_str) {
        Ok(s) => s,
        Err(_e) => {
            return;
        }
    };

    unsafe { __android_log_write(priority as i32, tag.as_ptr(), msg.as_ptr()) };
}

pub fn panic_hook(backtrace: bool) -> Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static> {
    // Mostly taken from https://doc.rust-lang.org/src/std/panicking.rs.html
    Box::new(move |info| {
        let location = info.location().unwrap();
        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        };

        log_error(format!(
            "Panicked at '{}', {} {} {}",
            location.file(),
            location.line(),
            location.column(),
            msg
        ));
        if backtrace {
            log_error(format!("Backtrace: {:#?}", Backtrace::force_capture()));
        }
    })
}

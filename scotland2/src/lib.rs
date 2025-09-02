use ctor::ctor;
use paper2::logger::LoggerConfig;
use std::backtrace::Backtrace;
use std::panic::PanicHookInfo;
use std::{ffi::CStr, path::PathBuf};

use mimalloc::MiMalloc;

// using mimalloc for android
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn log_info(message_str: String) {
    #[cfg(target_os = "android")]
    paper2::logger::logcat_logger::log_info(message_str);
}
fn log_error(message_str: String) {
    #[cfg(target_os = "android")]
    paper2::logger::logcat_logger::log_error(message_str);
}

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

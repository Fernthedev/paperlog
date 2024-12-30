use std::{ffi::CStr, path::PathBuf};

use ctor::ctor;
use paper2::LoggerConfig;

#[ctor]
fn dlopen_initialize() {
    println!("DLOpen initializing");

    let id = unsafe { CStr::from_ptr(scotland2::modloader_get_application_id()) }.to_string_lossy();
    let path = PathBuf::from(format!("/sdcard/ModData/{id}/logs2",));
    let config = LoggerConfig {
        context_log_path: path.clone(),
        ..LoggerConfig::default()
    };

    let file = path.join("Paperlog.log");
    if let Err(e) = paper2::init_logger(config, file) {
        eprintln!("Error occurred in logging thread: {}", e);
        panic!("Error occurred in logging thread: {}", e);
    }
}

use std::ffi::CStr;

use ctor::ctor;
use paper2::LoggerConfig;

mod scotland2;

#[ctor]
fn dlopen_initialize() {
    println!("DLOpen initializing");

    let id = unsafe { CStr::from_ptr(scotland2::modloader_get_application_id()) }.to_string_lossy();
    let path = format!("/sdcard/ModData/{id}/logs2",);

    if let Err(e) = paper2::init_logger(LoggerConfig::default(), path.into()) {
        eprintln!("Error occurred in logging thread: {}", e);
        panic!("Error occurred in logging thread: {}", e);
    }
}

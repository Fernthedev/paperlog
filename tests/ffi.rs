#![cfg(feature = "ffi")]

use std::ffi::{c_void, CString};
use std::sync::Mutex;
use std::{fs, path::PathBuf, thread, time::Duration};

use paper2::ffi::{
    paper2_add_log_sink, paper2_get_inited, paper2_init_logger_ffi, paper2_queue_log_ffi,
    paper2_register_context_id, paper2_unregister_context_id, paper2_wait_flush_timeout,
    LogDataC,
};
use paper2::types::log_level::LogLevel;

static CAPTURED_MESSAGES: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

unsafe extern "C" fn capture_sink(log_data: *const LogDataC, _user_data: *mut c_void) {
    let data = unsafe { &*log_data };
    let bytes = unsafe { data.message.as_bytes() }.to_vec();
    CAPTURED_MESSAGES.lock().unwrap().push(bytes);
}

// `paper2_init_logger_ffi` populates a process-global `OnceLock`, so only
// the first call in this whole test binary actually takes effect. Every
// assertion that depends on that shared global state is kept in this one
// test function rather than spread across tests that could race for who
// "wins" the init.
#[test]
fn ffi_round_trip() {
    assert!(
        !unsafe { paper2_get_inited() },
        "should not be inited before the first init call in this process"
    );

    let log_dir = PathBuf::from("./logs/ffi");
    let log_path = log_dir.join("ffi_test.log");
    let path_c = CString::new(log_path.to_str().unwrap()).unwrap();

    let ok = unsafe { paper2_init_logger_ffi(std::ptr::null(), path_c.as_ptr()) };
    assert!(ok, "paper2_init_logger_ffi should succeed");
    assert!(unsafe { paper2_get_inited() });

    // A second init attempt is a no-op (the global logger only initializes
    // once) and must not panic or otherwise misbehave.
    let ok_again = unsafe { paper2_init_logger_ffi(std::ptr::null(), path_c.as_ptr()) };
    assert!(ok_again);

    unsafe { paper2_add_log_sink(capture_sink, std::ptr::null_mut()) };

    let file_c = CString::new(file!()).unwrap();

    // A normal, valid-UTF-8 log line.
    let message = CString::new("hello from ffi").unwrap();
    let queued = unsafe {
        paper2_queue_log_ffi(
            LogLevel::Info,
            std::ptr::null(),
            message.as_ptr(),
            file_c.as_ptr(),
            line!() as i32,
            column!() as i32,
            std::ptr::null(),
        )
    };
    assert!(queued);

    // A message containing raw, invalid-UTF-8 bytes. `CString` only rejects
    // interior NULs, not invalid UTF-8, so this is legal input from the C
    // side — and it must survive untouched, not get rejected or lossily
    // rewritten anywhere along the way.
    let invalid_bytes: Vec<u8> = vec![b'b', b'a', 0xFF, 0xFE, b'd'];
    let invalid_message = CString::new(invalid_bytes.clone()).unwrap();
    let queued_invalid = unsafe {
        paper2_queue_log_ffi(
            LogLevel::Warn,
            std::ptr::null(),
            invalid_message.as_ptr(),
            file_c.as_ptr(),
            line!() as i32,
            column!() as i32,
            std::ptr::null(),
        )
    };
    assert!(queued_invalid);

    // Registering/unregistering a context tag must not panic either, even
    // for a tag that isn't valid UTF-8.
    let tag = CString::new(vec![b'c', b't', 0xFF, b'x']).unwrap();
    unsafe { paper2_register_context_id(tag.as_ptr()) };
    unsafe { paper2_unregister_context_id(tag.as_ptr()) };

    thread::sleep(Duration::from_millis(50));
    unsafe { paper2_wait_flush_timeout(500) };

    let captured = CAPTURED_MESSAGES.lock().unwrap();
    assert!(
        captured.iter().any(|m| m == b"hello from ffi"),
        "sink never received the plain message; got {captured:?}"
    );
    assert!(
        captured.iter().any(|m| *m == invalid_bytes),
        "sink didn't receive the non-UTF-8 message byte-for-byte; got {captured:?}"
    );
    drop(captured);

    if cfg!(feature = "file") {
        let contents = fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("hello from ffi"));
    }
}

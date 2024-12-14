use crate::log_level::LogLevel;
use crate::{init_logger, ThreadSafeLoggerThread};
use std::ffi::CStr;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::ptr;


#[repr(C)]
pub struct ThreadSafeLoggerThreadFfi(*mut std::ffi::c_void);

#[no_mangle]
pub extern "C" fn init_logger_ffi(path: *const c_char) -> ThreadSafeLoggerThreadFfi {
    if path.is_null() {
        return ThreadSafeLoggerThreadFfi::null();
    }

    let c_str = unsafe { CStr::from_ptr(path) };
    let path_str = match c_str.to_str() {
        Ok(str) => str,
        Err(_) => return ThreadSafeLoggerThreadFfi::null(),
    };

    let path_buf = PathBuf::from(path_str);
    match init_logger(path_buf) {
        Ok(logger_thread) => Box::into_raw(Box::new(logger_thread)).into(),
        Err(_) => ThreadSafeLoggerThreadFfi::null(),
    }
}

#[no_mangle]
pub extern "C" fn queue_log_ffi(
    logger_thread: ThreadSafeLoggerThreadFfi,
    level: LogLevel,
    tag: *const c_char,
    message: *const c_char,
    file: *const c_char,
    line: c_int,
) {
    if logger_thread.is_null() || message.is_null() || file.is_null() {
        return;
    }

    let tag = unsafe {
        tag.as_ref()
            .map(|c_str| CStr::from_ptr(c_str))
            .map(|c| c.to_string_lossy().into_owned())
    };

    let message = unsafe { CStr::from_ptr(message).to_string_lossy().into_owned() };
    let file = unsafe { CStr::from_ptr(file).to_string_lossy().into_owned() };

    logger_thread
        .read()
        .unwrap()
        .queue_log(level, tag, message, file, line as u32);
}

#[no_mangle]
pub extern "C" fn wait_for_flush(logger_thread: ThreadSafeLoggerThreadFfi) -> bool {
    if logger_thread.is_null() {
        return false;
    }

    logger_thread.read().unwrap().wait_for_flush();

    true
}

#[no_mangle]
pub extern "C" fn get_inited(logger_thread: ThreadSafeLoggerThreadFfi) -> bool {
    if logger_thread.is_null() {
        return false;
    }

    logger_thread
        .read()
        .unwrap()
        .is_inited()
        .load(std::sync::atomic::Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn wait_flush_timeout(
    logger_thread: ThreadSafeLoggerThreadFfi,
    timeout_ms: c_int,
) -> bool {
    if logger_thread.is_null() {
        return false;
    }

    logger_thread
        .read()
        .unwrap()
        .wait_for_flush_timeout(std::time::Duration::from_millis(timeout_ms as u64));

    true
}
#[no_mangle]
pub extern "C" fn free_logger_thread(logger_thread: ThreadSafeLoggerThreadFfi) {
    if logger_thread.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(logger_thread.into()));
    }
}

impl From<*mut ThreadSafeLoggerThread> for ThreadSafeLoggerThreadFfi {
    fn from(ptr: *mut ThreadSafeLoggerThread) -> Self {
        Self(ptr as *mut std::ffi::c_void)
    }
}

impl From<ThreadSafeLoggerThreadFfi> for *mut ThreadSafeLoggerThread {
    fn from(ffi: ThreadSafeLoggerThreadFfi) -> Self {
        ffi.0 as *mut ThreadSafeLoggerThread
    }
}

impl Deref for ThreadSafeLoggerThreadFfi {
    type Target = ThreadSafeLoggerThread;

    fn deref(&self) -> &Self::Target {
        if self.0.is_null() {
            panic!("Attempted to dereference a null pointer");
        }

        unsafe { &*(self.0 as *mut ThreadSafeLoggerThread) }
    }
}
impl DerefMut for ThreadSafeLoggerThreadFfi {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.0.is_null() {
            panic!("Attempted to dereference a null pointer");
        }

        unsafe { &mut *(self.0 as *mut ThreadSafeLoggerThread) }
    }
}

impl ThreadSafeLoggerThreadFfi {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
    pub fn null() -> Self {
        Self(ptr::null_mut())
    }
}

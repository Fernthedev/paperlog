use std::ffi::{c_char, CString};

/// Helper struct to manage ownership of C strings across FFI boundaries.
/// I'm not sure if I can pass a struct using CString directly, so this is a workaround.
#[repr(transparent)]
pub struct OwnedCStr(*const c_char);

impl From<&str> for OwnedCStr {
    fn from(s: &str) -> Self {
        let c_string = CString::new(s).expect("CString::new failed");
        let ptr = c_string.into_raw();
        Self (ptr)
    }
}
impl From<String> for OwnedCStr {
    fn from(s: String) -> Self {
        let c_string = CString::new(s).expect("CString::new failed");
        let ptr = c_string.into_raw();
        Self (ptr)
    }
}

impl<T> From<Option<T>> for OwnedCStr where T: AsRef<str> {
    fn from(s: Option<T>) -> Self {
        match s {
            Some(str) => str.as_ref().into(),
            None => Self(std::ptr::null()),
        }
    }
}

impl Drop for OwnedCStr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                drop(CString::from_raw(self.0 as *mut c_char));
            }
        }
    }
}

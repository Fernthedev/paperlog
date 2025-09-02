use std::ffi::{c_char, CString};

/// Helper struct to manage ownership of C strings across FFI boundaries.
/// I'm not sure if I can pass a struct using CString directly, so this is a workaround.
#[repr(C)]
pub struct StringRef<'a>(*const u8, usize, std::marker::PhantomData<&'a str>);

impl<'a> From<&'a str> for StringRef<'a> {
    fn from(s: &'a str) -> Self {
        Self(s.as_ptr(), s.len(), std::marker::PhantomData::default())
    }
}

impl<'a> From<Option<&'a str>> for StringRef<'a> {
    fn from(s: Option<&'a str>) -> Self {
        match s {
            Some(str) => StringRef(str.as_ptr(), str.len(), std::marker::PhantomData::default()),
            None => StringRef(std::ptr::null(), 0, std::marker::PhantomData::default()),
        }
    }
}

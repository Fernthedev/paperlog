use crate::types::PaperStr;

/// Helper struct to manage ownership of a byte slice across FFI boundaries.
/// I'm not sure if I can pass a struct using CString directly, so this is a workaround.
///
/// Deliberately a raw `(ptr, len)` pair over arbitrary bytes rather than a
/// `&str` — the data ultimately comes from a caller-supplied C string that
/// isn't guaranteed to be valid UTF-8, and re-validating it here would mean
/// either rejecting or lossily rewriting bytes the caller explicitly wrote.
#[repr(C)]
pub struct StringRef<'a>(*const u8, usize, std::marker::PhantomData<&'a [u8]>);

impl<'a> StringRef<'a> {
    /// Reinterprets the `(pointer, length)` pair as a byte slice.
    ///
    /// # Safety
    /// The pointed-to memory must still be valid and unchanged since this
    /// `StringRef` was constructed — the same requirement a C consumer
    /// reading the raw `_0`/`_1` fields directly is already bound by.
    pub unsafe fn as_bytes(&self) -> &'a [u8] {
        if self.0.is_null() {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.0, self.1) }
        }
    }
}

impl<'a> From<&'a PaperStr> for StringRef<'a> {
    fn from(s: &'a PaperStr) -> Self {
        Self(s.as_ptr(), s.len(), std::marker::PhantomData)
    }
}

impl<'a> From<Option<&'a PaperStr>> for StringRef<'a> {
    fn from(s: Option<&'a PaperStr>) -> Self {
        match s {
            Some(s) => s.into(),
            None => StringRef(std::ptr::null(), 0, std::marker::PhantomData),
        }
    }
}

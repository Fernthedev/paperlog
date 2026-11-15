use std::{fmt, ops::Deref};

use bstr::ByteSlice;
use smallvec::SmallVec;

use crate::types::{PaperStr, PaperString};

/// Tags up to this many bytes are stored inline, with no heap allocation.
/// Tags are normally short mod/context names, so this covers nearly all of
/// them; longer ones transparently spill to the heap.
const INLINE_TAG_BYTES: usize = 24;

/// An owned log tag (context name).
///
/// Every queued log carries its own tag, so storing it inline (small-string
/// optimization) saves one heap allocation per log line on the producer
/// side, and the matching free on the logging thread. Like the rest of the
/// crate, a tag is arbitrary bytes and isn't required to be valid UTF-8.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct PaperTag(SmallVec<[u8; INLINE_TAG_BYTES]>);

impl PaperTag {
    pub fn new(bytes: &[u8]) -> Self {
        Self(SmallVec::from_slice(bytes))
    }

    pub fn as_bstr(&self) -> &PaperStr {
        self.0.as_bstr()
    }
}

impl Deref for PaperTag {
    type Target = PaperStr;

    fn deref(&self) -> &PaperStr {
        self.as_bstr()
    }
}

impl AsRef<[u8]> for PaperTag {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for PaperTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_bstr(), f)
    }
}

impl fmt::Display for PaperTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_bstr(), f)
    }
}

impl From<&[u8]> for PaperTag {
    fn from(bytes: &[u8]) -> Self {
        Self::new(bytes)
    }
}

impl From<&str> for PaperTag {
    fn from(s: &str) -> Self {
        Self::new(s.as_bytes())
    }
}

impl From<&PaperStr> for PaperTag {
    fn from(s: &PaperStr) -> Self {
        Self::new(s)
    }
}

impl From<String> for PaperTag {
    fn from(s: String) -> Self {
        Self::from(s.into_bytes())
    }
}

impl From<Vec<u8>> for PaperTag {
    fn from(v: Vec<u8>) -> Self {
        // `SmallVec::from_vec` keeps an existing heap buffer as-is; only
        // copy when the tag actually fits inline.
        if v.len() <= INLINE_TAG_BYTES {
            Self::new(&v)
        } else {
            Self(SmallVec::from_vec(v))
        }
    }
}

impl From<PaperString> for PaperTag {
    fn from(s: PaperString) -> Self {
        Self::from(Vec::from(s))
    }
}

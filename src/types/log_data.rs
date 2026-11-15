use std::time::SystemTime;

use bstr::ByteSlice;

use crate::{
    types::log_level::LogLevel,
    types::{logger_chunk::LogDataChunk, PaperStr, PaperString, PaperTag},
};

pub const DEFAULT_TAG: &str = "GLOBAL";

#[derive(Debug, Clone)]
pub struct LogData {
    pub level: LogLevel,
    pub tag: Option<PaperTag>,
    pub message: PaperString,
    /// When the log was created. Kept as a raw `SystemTime` so creating a
    /// log is a cheap clock read; conversion to local time happens on the
    /// logging thread, when the line is formatted.
    pub timestamp: SystemTime,

    pub file: &'static PaperStr,
    pub line: u32,
    pub column: u32,
    pub function_name: Option<&'static PaperStr>,
}

impl LogData {
    pub fn new(
        level: LogLevel,
        tag: Option<PaperTag>,
        message: PaperString,
        file: &'static PaperStr,
        line: u32,
        column: u32,
        function_name: Option<&'static PaperStr>,
    ) -> Self {
        Self {
            level,
            tag,
            message,
            timestamp: SystemTime::now(),
            file,
            line,
            column,
            function_name,
        }
    }

    /// Borrows the tag as bytes, falling back to [`DEFAULT_TAG`] without allocating.
    pub(crate) fn tag_or_default(&self) -> &PaperStr {
        self.tag
            .as_deref()
            .unwrap_or_else(|| PaperStr::new(DEFAULT_TAG))
    }

    pub(crate) fn function_name_or_default(&self) -> &PaperStr {
        self.function_name
            .unwrap_or_else(|| PaperStr::new("default"))
    }

    /// Borrows the whole entry as a single, unsplit [`LogDataChunk`].
    fn as_chunk(&self) -> LogDataChunk<'_> {
        LogDataChunk {
            original: self,
            message: self.message.as_bstr(),
        }
    }

    /// Formats the whole entry. See [`LogDataChunk::format`], which does the
    /// actual work — one line/chunk at a time is the only place formatting
    /// needs to happen, so `LogData` just delegates to a chunk of itself.
    pub fn format(&self) -> String {
        self.as_chunk().format()
    }

    pub fn write_to_io(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.as_chunk().write_to_io(writer)
    }

    pub fn write_compact_to_io(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.as_chunk().write_compact_to_io(writer)
    }
}
impl Default for LogData {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            tag: None,
            message: PaperString::default(),
            timestamp: SystemTime::now(),
            file: PaperStr::new(&[]),
            line: 0,
            column: 0,
            function_name: None,
        }
    }
}

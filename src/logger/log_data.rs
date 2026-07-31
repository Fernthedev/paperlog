use bstr::ByteSlice;
use chrono::{DateTime, Local};

use crate::{log_level::LogLevel, logger::logger_chunk::LogDataChunk, types::{PaperStr, PaperString}};

pub const DEFAULT_TAG: &str = "GLOBAL";

#[derive(Debug, Clone)]
pub struct LogData {
    pub level: LogLevel,
    pub tag: Option<PaperString>,
    pub message: PaperString,
    pub timestamp: DateTime<Local>,

    pub file: &'static PaperStr,
    pub line: u32,
    pub column: u32,
    pub function_name: Option<&'static PaperStr>,
}

impl LogData {
    pub fn new(
        level: LogLevel,
        tag: Option<PaperString>,
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
            timestamp: Local::now(),
            file,
            line,
            column,
            function_name,
        }
    }

    /// Borrows the tag as bytes, falling back to [`DEFAULT_TAG`] without allocating.
    ///
    /// `PaperString` (`BString`) derefs to `Vec<u8>`, not `PaperStr`, so this
    /// goes through `as_bstr()` rather than `as_deref()`.
    pub(crate) fn tag_or_default(&self) -> &PaperStr {
        self.tag.as_ref().map_or_else(|| PaperStr::new(DEFAULT_TAG), |t| t.as_bstr())
    }

    pub(crate) fn function_name_or_default(&self) -> &PaperStr {
        self.function_name.unwrap_or_else(|| PaperStr::new("default"))
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
            timestamp: Local::now(),
            file: PaperStr::new(&[]),
            line: 0,
            column: 0,
            function_name: None,
        }
    }
}

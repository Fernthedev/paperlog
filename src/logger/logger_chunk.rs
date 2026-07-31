use bstr::ByteSlice;

use crate::{logger::LogData, types::PaperStr};



#[derive(Debug, Clone, Copy)]
pub struct LogDataChunk<'a> {
    pub original: &'a LogData,
    pub message: &'a PaperStr,
}


impl<'a> LogDataChunk<'a> {
    pub fn to_owned(&self) -> LogData {
        LogData {
            message: self.message.to_owned(),
            ..self.original.clone()
        }
    }

    /// Borrows the tag as bytes, falling back to the default tag. See
    /// [`LogData::tag_or_default`].
    pub fn tag(&self) -> &'a PaperStr {
        self.original.tag_or_default()
    }

    pub fn format(&self) -> String {
        format!(
            "{level} {time} [{tag}] [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.original.level,
            time = self.original.timestamp.format("%Y-%m-%d %H:%M:%S"),
            tag = self.tag(),
            message = self.message,
            line = self.original.line,
            column = self.original.column,
            file = self.original.file,
            function_name = self.original.function_name_or_default(),
        )
    }

    pub fn write_to_io(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(
            writer,
            "{level} {time} [{tag}] [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.original.level.short(),
            time = self.original.timestamp.format("%Y-%m-%d %H:%M:%S"),
            tag = self.tag(),
            message = self.message,
            line = self.original.line,
            column = self.original.column,
            file = self.original.file,
            function_name = self.original.function_name_or_default(),
        )
    }

    pub fn write_compact_to_io(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        writeln!(
            writer,
            "{level} {time} [{file}:{line}:{column} @ {function_name}] {message}",
            level = self.original.level.short(),
            time = self.original.timestamp.format("%Y-%m-%d %H:%M:%S"),
            message = self.message,
            line = self.original.line,
            column = self.original.column,
            file = self.original.file,
            function_name = self.original.function_name_or_default(),
        )
    }
}

/// Helper to chunk a BStr by character count instead of raw byte count
fn chunk_by_chars<'a>(s: &'a PaperStr, max_chars: usize) -> impl Iterator<Item = &'a PaperStr> {
    let mut remaining = s;

    std::iter::from_fn(move || {
        if remaining.is_empty() {
            return None;
        }

        // Find the byte offset corresponding to `max_chars` UTF-8 characters.
        // `char_indices()` yields (start_byte_idx, end_byte_idx, char)
        let byte_offset = remaining
            .char_indices()
            .nth(max_chars)
            .map(|(start, _, _)| start)
            .unwrap_or(remaining.len());

        let (chunk, tail) = remaining.split_at(byte_offset);
        remaining = tail.as_bstr();
        Some(chunk.as_bstr())
    })
}

/// Split log message by line endings and then split each line into character-aligned chunks
pub fn split_str_into_chunks(
    queue: &[LogData],
    max_chars: usize,
) -> impl Iterator<Item = LogDataChunk<'_>> + '_ {
    queue.iter().flat_map(move |log| {
        log.message
            .lines() // Splits by line endings (\n or \r\n) safely
            .flat_map(move |line| {
                chunk_by_chars(line.as_bstr(), max_chars).map(move |chunk| LogDataChunk {
                    original: log,
                    message: chunk,
                })
            })
    })
}
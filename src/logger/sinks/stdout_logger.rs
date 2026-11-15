use crate::types::logger_chunk::LogDataChunk;

/// Writes one line to `out`. The logging thread passes a buffered,
/// already-locked stdout so a whole batch costs one lock and one write;
/// errors are returned (and reported) rather than panicking like `println!`.
pub(crate) fn do_log(log: LogDataChunk<'_>, out: &mut impl std::io::Write) -> std::io::Result<()> {
    writeln!(out, "{log}")
}

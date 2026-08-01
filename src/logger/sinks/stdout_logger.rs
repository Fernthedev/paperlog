use crate::types::logger_chunk::LogDataChunk;

pub(crate) fn do_log(log: LogDataChunk<'_>) {
    println!("{}", log.format());
}

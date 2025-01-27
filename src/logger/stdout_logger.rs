use super::LogData;

// assert tracing is not enabled
#[cfg(not(feature = "tracing"))]
compile_error!("The 'tracing' feature must be enabled to use this logger.");

pub(crate) fn do_log(log: &LogData) {
    println!("{}", log.format());
}

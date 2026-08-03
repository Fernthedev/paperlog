#[cfg(feature = "logcat")]
pub mod logcat_logger;

#[cfg(feature = "file")]
pub mod file_logger;

#[cfg(feature = "stdout")]
pub mod stdout_logger;

#[cfg(feature = "sinks")]
pub mod sink_logger;

#[cfg(feature = "tracing")]
pub mod tracing_logger;

pub mod config;
pub mod log_level;
pub mod logger_chunk;

// export LogData for FFI use
mod log_data;

pub use log_data::LogData;

/// A string type for paper data.
pub type PaperString = bstr::BString;
pub type PaperStr = bstr::BStr;

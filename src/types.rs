pub mod config;
pub mod file_logger_map;
pub mod log_level;
pub mod logger_chunk;

// export LogData for FFI use
mod log_data;
mod tag;
pub(crate) mod time;

pub use log_data::LogData;
pub use tag::PaperTag;

/// A string type for paper data.
pub type PaperString = bstr::BString;
pub type PaperStr = bstr::BStr;

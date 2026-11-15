use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
};

use rustc_hash::FxHashMap;

use crate::types::{PaperStr, PaperString};

/// Holds the global log file plus one file per registered context.
///
/// This type does no locking itself; it is guarded by a single
/// `Mutex<FileLoggers>` in [`crate::logger::SharedLoggerState`]. Only the
/// logging thread writes on the hot path (once per batch), so one lock is
/// enough — `add_context`/`remove_context` and the panic-hook fallback are
/// rare enough that per-file locking would buy nothing.
pub struct FileLoggers {
    base_path: std::path::PathBuf,

    global_file: BufWriter<File>,
    map: FxHashMap<PaperString, BufWriter<File>>,
}

impl FileLoggers {
    pub fn new(
        log_file: std::path::PathBuf,
        context_log_path: std::path::PathBuf,
    ) -> io::Result<Self> {
        fs::create_dir_all(&context_log_path)?;

        if let Some(parent) = log_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let global_file = BufWriter::new(File::create(&log_file)?);

        Ok(Self {
            base_path: context_log_path,
            global_file,
            map: FxHashMap::default(),
        })
    }

    pub fn add_context(&mut self, context: &PaperStr) -> io::Result<()> {
        let file_path = self.base_path.join(format!("{}.log", context));
        let writer = BufWriter::new(File::create(file_path)?);
        self.map.insert(context.into(), writer);
        Ok(())
    }

    pub fn remove_context(&mut self, context: &PaperStr) {
        self.map.remove(context);
    }

    /// Returns the global writer and, if `context` is registered, its
    /// context writer. Both are borrowed mutably at once (disjoint fields).
    pub fn writers_for(&mut self, context: Option<&PaperStr>) -> Writers<'_> {
        Writers {
            global: &mut self.global_file,
            context: context.and_then(|tag| self.map.get_mut(tag)),
        }
    }

    /// Flushes the global file and every context file.
    pub fn flush(&mut self) -> io::Result<()> {
        self.global_file.flush()?;
        self.map.values_mut().try_for_each(|file| file.flush())
    }
}

/// The writers a single log line goes to. See [`FileLoggers::writers_for`].
pub struct Writers<'a> {
    pub global: &'a mut BufWriter<File>,
    pub context: Option<&'a mut BufWriter<File>>,
}

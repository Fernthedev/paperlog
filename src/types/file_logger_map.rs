use std::{
    fs::{self, File},
    io::{self, BufWriter},
};

use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::types::{PaperStr, PaperString};

/// Holds the global log file plus one file per registered context.
///
/// Each file is behind its own [`Mutex`] rather than a single lock over the
/// whole map, so writing to the global file and writing to a context file
/// never contend with each other. The outer container (`RwLock<FileLoggers>`
/// in [`crate::logger::SharedLoggerState`]) only needs a *read* lock for the
/// hot logging path — a *write* lock is only required when the context map
/// itself is structurally changed (`add_context`/`remove_context`).
pub struct FileLoggers {
    base_path: std::path::PathBuf,

    global_file: Mutex<BufWriter<File>>,
    map: FxHashMap<PaperString, Mutex<BufWriter<File>>>,
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

        let global_file = {
            let inner = File::create(&log_file)?;
            BufWriter::new(inner)
        };

        Ok(Self {
            base_path: context_log_path,
            global_file: Mutex::new(global_file),
            map: FxHashMap::default(),
        })
    }

    pub fn add_context(&mut self, context: &PaperStr) -> std::io::Result<()> {
        let file_path = self.base_path.join(format!("{}.log", context));
        let file = File::create(file_path)?;
        let writer = BufWriter::new(file);
        self.map.insert(context.into(), Mutex::new(writer));
        Ok(())
    }

    pub fn get_writer(&self, context: &PaperStr) -> Option<&Mutex<BufWriter<File>>> {
        self.map.get(context)
    }

    pub fn remove_context(&mut self, context: &PaperStr) {
        self.map.remove(context);
    }

    pub fn get_global_file(&self) -> &Mutex<BufWriter<File>> {
        &self.global_file
    }

    pub fn get_context_map(&self) -> &FxHashMap<PaperString, Mutex<BufWriter<File>>> {
        &self.map
    }
}

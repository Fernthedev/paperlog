#![cfg(feature = "file")]

use std::os::unix::ffi::OsStrExt;
use std::{ffi::OsStr, fs, path::PathBuf, thread, time::Duration};

use paper2::logger::LoggerThreadCtx;
use paper2::types::config::LoggerConfig;
use paper2::types::log_level::LogLevel;
use paper2::types::{LogData, PaperStr};

// Context tags route to per-tag log files named after the tag. Since file
// names on Linux are just byte strings (not required to be valid UTF-8),
// and `add_context` takes a `&PaperStr` rather than a `&str` specifically so
// non-UTF-8 tags aren't rejected or rewritten, a tag containing an invalid
// UTF-8 byte should still produce a real file with that exact byte sequence
// as its name.
//
// This needs a genuinely POSIX filesystem: some mounts (NTFS/exFAT via
// FUSE, common for a shared drive) transcode names to UTF-16 and reject
// invalid UTF-8 with EILSEQ, which isn't representative of where paper2
// actually logs to on Android (internal storage, ext4/f2fs). Use the OS
// temp dir rather than a repo-relative path so this doesn't depend on
// which filesystem the checkout happens to live on.
#[test]
fn non_utf8_tag_routes_to_a_byte_exact_context_file() {
    let context_log_path = std::env::temp_dir().join("paper2_non_utf8_test");
    let config = LoggerConfig {
        max_string_len: 1024,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: context_log_path.clone(),
    };
    let log_path = context_log_path.join("global.log");

    let mut logger = LoggerThreadCtx::new(config, log_path.clone()).unwrap();

    let tag_bytes: Vec<u8> = vec![b'c', b'x', 0xFF, b't'];
    let tag = PaperStr::new(&tag_bytes);
    logger.add_context(tag).unwrap();

    let expected_path = context_log_path
        .join(OsStr::from_bytes(&tag_bytes))
        .with_extension("log");
    assert!(
        expected_path.exists(),
        "expected a context file at {expected_path:?}"
    );

    let logger = logger.init(false).unwrap();
    logger.read().queue_log(LogData {
        level: LogLevel::Info,
        tag: Some(tag.to_owned()),
        message: "hello from a byte-tagged context".into(),
        file: file!().into(),
        line: line!(),
        column: column!(),
        ..Default::default()
    });

    thread::sleep(Duration::from_millis(50));
    logger
        .read()
        .wait_for_flush_timeout(Duration::from_millis(500));

    let contents = fs::read_to_string(&expected_path).unwrap();
    assert!(contents.contains("hello from a byte-tagged context"));

    logger.write().remove_context(tag);
    let _ = fs::remove_file(&expected_path);
    let _ = fs::remove_file(&log_path);
    let _ = fs::remove_dir(&context_log_path);
}

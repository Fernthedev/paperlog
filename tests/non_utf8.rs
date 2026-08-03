#![cfg(feature = "file")]

use std::{fs, thread, time::Duration};

use paper2::types::config::LoggerConfig;
use paper2::types::log_level::LogLevel;
use paper2::types::{LogData, PaperStr};

// Context tags may contain arbitrary bytes (`add_context` takes a `&PaperStr`
// rather than a `&str` so non-UTF-8 tags aren't rejected outright), but the
// on-disk context file name is derived via `PaperStr`'s lossy `Display` impl,
// which replaces invalid UTF-8 with U+FFFD — so file names stay valid UTF-8
// even when the tag itself isn't. Routing of a log line to the right context
// file is still keyed by the exact tag bytes (a `FxHashMap<PaperString, _>`
// lookup), independent of how the file got named on disk.
//
// Use the OS temp dir rather than a repo-relative path so this doesn't
// depend on which filesystem the checkout happens to live on.
#[test]
fn non_utf8_tag_routes_to_a_lossily_named_context_file() {
    let context_log_path = std::env::temp_dir().join("paper2_non_utf8_test");
    let log_path = context_log_path.join("global.log");
    let config = LoggerConfig {
        max_string_len: 1024,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: context_log_path.clone(),
        log_path: log_path.clone(),
    };

    let logger = config.init(false).unwrap();

    let tag_bytes: Vec<u8> = vec![b'c', b'x', 0xFF, b't'];
    let tag = PaperStr::new(&tag_bytes);
    logger.add_context(tag).unwrap();

    // Matches the lossy `Display` conversion `add_context` uses for the file
    // name: the invalid `0xFF` byte becomes a single U+FFFD.
    let expected_path = context_log_path.join(format!("{tag}.log"));
    assert!(
        expected_path.exists(),
        "expected a context file at {expected_path:?}"
    );

    logger.queue_log(LogData {
        level: LogLevel::Info,
        tag: Some(tag.to_owned()),
        message: "hello from a byte-tagged context".into(),
        file: file!().into(),
        line: line!(),
        column: column!(),
        ..Default::default()
    });

    thread::sleep(Duration::from_millis(50));
    logger.wait_for_flush_timeout(Duration::from_millis(500));

    let contents = fs::read_to_string(&expected_path).unwrap();
    assert!(contents.contains("hello from a byte-tagged context"));

    logger.remove_context(tag);
    let _ = fs::remove_file(&expected_path);
    let _ = fs::remove_file(&log_path);
    let _ = fs::remove_dir(&context_log_path);
}

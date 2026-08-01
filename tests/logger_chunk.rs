use paper2::types::log_level::LogLevel;
use paper2::types::logger_chunk::split_str_into_chunks;
use paper2::types::{LogData, PaperStr};

fn make_log(message: impl Into<paper2::types::PaperString>) -> LogData {
    LogData {
        level: LogLevel::Info,
        message: message.into(),
        ..Default::default()
    }
}

// `.to_vec()` here is the inherent `[u8]` method (available via `PaperStr`'s
// deref to `[u8]`), not `bstr::ByteSlice::as_bytes()` — integration tests
// only see `paper2`'s public API and its `[dev-dependencies]`, not its
// ordinary `[dependencies]` like `bstr`, so trait methods from `bstr` aren't
// reachable here without adding it as a dev-dependency too.
fn chunk_bytes(queue: &[LogData], max_chars: usize) -> Vec<Vec<u8>> {
    split_str_into_chunks(queue, max_chars)
        .map(|chunk| chunk.message.to_vec())
        .collect()
}

#[test]
fn short_message_is_not_split() {
    let queue = vec![make_log("hello world")];
    let chunks = chunk_bytes(&queue, 100);

    assert_eq!(chunks, vec![b"hello world".to_vec()]);
}

#[test]
fn empty_message_produces_no_chunks() {
    let queue = vec![make_log("")];
    let chunks = chunk_bytes(&queue, 100);

    assert!(chunks.is_empty());
}

#[test]
fn long_message_splits_by_char_count_and_reassembles() {
    let message = "a".repeat(250);
    let queue = vec![make_log(message.clone())];
    let chunks = chunk_bytes(&queue, 100);

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].len(), 100);
    assert_eq!(chunks[1].len(), 100);
    assert_eq!(chunks[2].len(), 50);

    let joined: Vec<u8> = chunks.concat();
    assert_eq!(joined, message.into_bytes());
}

#[test]
fn multiline_message_splits_by_line_first() {
    let queue = vec![make_log("line one\nline two\nline three")];
    let chunks = chunk_bytes(&queue, 100);

    assert_eq!(
        chunks,
        vec![
            b"line one".to_vec(),
            b"line two".to_vec(),
            b"line three".to_vec(),
        ]
    );
}

#[test]
fn crlf_line_endings_are_split_too() {
    let queue = vec![make_log("first\r\nsecond")];
    let chunks = chunk_bytes(&queue, 100);

    assert_eq!(chunks, vec![b"first".to_vec(), b"second".to_vec()]);
}

#[test]
fn multiple_log_entries_are_each_chunked_independently() {
    let queue = vec![make_log("one"), make_log("two"), make_log("three")];
    let chunks = chunk_bytes(&queue, 100);

    assert_eq!(
        chunks,
        vec![b"one".to_vec(), b"two".to_vec(), b"three".to_vec()]
    );
}

#[test]
fn multibyte_utf8_chars_are_never_split_mid_codepoint() {
    // Each "€" is 3 bytes; chunking by *chars* (not bytes) must never slice
    // through the middle of one, even when `max_chars` doesn't divide evenly.
    let message = "€".repeat(150);
    let queue = vec![make_log(message.clone())];
    let chunks = chunk_bytes(&queue, 100);

    for chunk in &chunks {
        assert!(
            std::str::from_utf8(chunk).is_ok(),
            "chunk was split mid-codepoint: {chunk:?}"
        );
    }

    let joined: Vec<u8> = chunks.concat();
    assert_eq!(joined, message.into_bytes());
}

#[test]
fn non_utf8_message_chunks_without_panicking_and_preserves_bytes() {
    // Raw, invalid-UTF-8 bytes (as could arrive unvalidated from the C API)
    // must still chunk without panicking, and every original byte must
    // survive the round trip untouched (no lossy replacement of the actual
    // data — that's only ever applied when *displaying* text).
    let invalid: Vec<u8> = vec![0xFF, 0xFE, b'h', b'i', 0xC0, 0xAF, b'!'];
    let queue = vec![LogData {
        level: LogLevel::Info,
        message: invalid.clone().into(),
        ..Default::default()
    }];
    let chunks = chunk_bytes(&queue, 2);

    let joined: Vec<u8> = chunks.concat();
    assert_eq!(joined, invalid);
}

#[test]
fn tag_falls_back_to_default_when_absent() {
    let queue = vec![make_log("hi")];
    let chunks: Vec<_> = split_str_into_chunks(&queue, 100).collect();

    assert_eq!(chunks[0].tag(), PaperStr::new("GLOBAL"));
}

#[test]
fn tag_is_preserved_when_present() {
    let log = LogData {
        tag: Some("MyTag".into()),
        ..make_log("hi")
    };
    let queue = vec![log];
    let chunks: Vec<_> = split_str_into_chunks(&queue, 100).collect();

    assert_eq!(chunks[0].tag(), PaperStr::new("MyTag"));
}

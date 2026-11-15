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

/// Reference chunking: split into groups of `max_chars` decoded characters,
/// using the same per-character byte ranges `bstr` reports (invalid bytes
/// included), so the fast paths can be checked against the plain algorithm.
fn reference_chunks(bytes: &[u8], max_chars: usize) -> Vec<Vec<u8>> {
    // Character boundaries come from chunking one character at a time
    // (integration tests can't reach `bstr` directly). With a limit of 1 the
    // ASCII fast paths are trivially right, and the multi-character
    // splitting being checked below isn't involved.
    let mut char_starts: Vec<usize> = Vec::new();
    let queue = vec![make_log(bytes.to_vec())];
    let mut offset = 0;
    for ch in chunk_bytes(&queue, 1) {
        char_starts.push(offset);
        offset += ch.len();
    }
    char_starts.push(offset);

    char_starts
        .iter()
        .step_by(max_chars)
        .zip(
            char_starts
                .iter()
                .skip(max_chars)
                .step_by(max_chars)
                .chain(std::iter::once(&offset)),
        )
        .filter(|(start, end)| start < end)
        .map(|(&start, &end)| bytes[start..end].to_vec())
        .collect()
}

#[test]
fn multibyte_char_at_exact_split_point_is_kept_whole() {
    // 3 ASCII chars then a 4-byte char: with max_chars = 4 the 4th char
    // (bytes 3..7) must stay in the first chunk.
    let message = "abc😀def";
    let queue = vec![make_log(message)];
    let chunks = chunk_bytes(&queue, 4);

    assert_eq!(chunks, vec!["abc😀".as_bytes().to_vec(), b"def".to_vec()]);
}

#[test]
fn ascii_prefix_then_multibyte_splits_by_chars() {
    // ASCII fast path covers the prefix, decoding resumes mid-chunk.
    let message = format!("{}{}", "a".repeat(5), "한".repeat(10));
    let queue = vec![make_log(message.clone())];
    let chunks = chunk_bytes(&queue, 8);

    let as_strs: Vec<&str> = chunks
        .iter()
        .map(|c| std::str::from_utf8(c).expect("split mid-codepoint"))
        .collect();
    assert_eq!(as_strs, vec!["aaaaa한한한", "한한한한한한한"]);
    assert_eq!(chunks.concat(), message.into_bytes());
}

#[test]
fn exactly_max_chars_of_ascii_is_one_chunk() {
    let message = "x".repeat(100);
    let queue = vec![make_log(message.clone())];
    assert_eq!(chunk_bytes(&queue, 100), vec![message.into_bytes()]);
}

#[test]
fn zero_max_chars_means_unlimited() {
    // Used to loop forever producing empty chunks.
    let queue = vec![make_log("hello\nworld")];
    assert_eq!(
        chunk_bytes(&queue, 0),
        vec![b"hello".to_vec(), b"world".to_vec()]
    );
}

#[test]
fn fast_paths_match_reference_chunking() {
    let inputs: Vec<Vec<u8>> = vec![
        "plain ascii text that is fairly long".into(),
        "£ ह € 한 😀 mixed ascii and multibyte £££".into(),
        "aaaaaaaa€".into(),
        "€aaaaaaaa".into(),
        vec![
            b'a', b'b', 0xFF, 0xE2, 0x82, b'c', 0xF0, 0x9F, 0x98, 0x80, b'd', 0xC0,
        ],
        vec![b'a'; 3]
            .into_iter()
            .chain([0xE2, 0x82])
            .chain(*b"zz")
            .collect(),
    ];

    for input in &inputs {
        for max_chars in 1..=12 {
            let queue = vec![make_log(input.clone())];
            let chunks = chunk_bytes(&queue, max_chars);
            assert_eq!(
                chunks,
                reference_chunks(input, max_chars),
                "input {input:?}, max_chars {max_chars}"
            );
            assert_eq!(&chunks.concat(), input);
        }
    }
}

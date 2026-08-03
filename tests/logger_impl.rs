use std::{thread, time::Duration};

use paper2::types::config::LoggerConfig;
use paper2::types::log_level::LogLevel;
use paper2::types::LogData;

fn test_config(name: &str) -> LoggerConfig {
    let context_log_path = std::env::temp_dir().join("paper").join(name);
    let log_path = context_log_path.join("test_log.log");

    LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path,
        log_path,
    }
}

#[test]
fn test_logger_thread_init() {
    let config = test_config("test_impl_8");

    let logger = config.init(false);
    assert!(logger.is_ok());
}

#[test]
fn test_queue_log() {
    let config = test_config("test_impl_queue");

    let logger = config.init(false).unwrap();
    logger.queue_log(LogData {
        level: LogLevel::Info,
        tag: Some("test".into()),
        message: "This is a test log".into(),
        file: file!().into(),
        line: line!(),
        column: column!(),
        ..Default::default()
    });

    thread::sleep(Duration::from_millis(500));
    logger.wait_for_flush_timeout(Duration::from_millis(500));

    assert_eq!(logger.queue_len(), 0);
}

#[test]
fn test_log_thread() {
    let config = test_config("test_impl_thread");

    let logger = config.init(false).unwrap();

    // `Logger` is cheap to clone (a channel `Sender` + one `Arc` refcount
    // bump), so no `Arc::clone` wrapper is needed here.
    let logger_clone = logger.clone();
    thread::spawn(move || {
        logger_clone.queue_log(LogData {
            level: LogLevel::Info,
            tag: Some("test".into()),
            message: "This is a test log".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            ..Default::default()
        });
    })
    .join()
    .unwrap();

    logger.wait_for_flush_timeout(Duration::from_millis(1000));
    assert_eq!(logger.queue_len(), 0);
}

use std::{fs, time::Duration};

use crate::{LoggerConfig, LoggerThread};

#[test]
fn test_logger_initialization() {
    let config = LoggerConfig {
        max_string_len: 1024,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: "./logs/10".into(),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path.clone()).unwrap();
    let thread = logger.init(false).unwrap();

    std::thread::sleep(Duration::from_millis(500));

    if cfg!(feature = "file") {
        assert!(log_path.exists());
        fs::remove_file(log_path).unwrap();
    }
}

#[test]
fn test_logger_config() {
    let config = LoggerConfig {
        max_string_len: 2048,
        log_max_buffer_count: 100,
        line_end: '\r',
        context_log_path: "./logs/1".into(),
    };
    let log_path = config.context_log_path.join("test_log.log");

    assert_eq!(config.max_string_len, 2048);
    assert_eq!(config.log_max_buffer_count, 100);
    assert_eq!(config.line_end, '\r');
}

#[test]
fn test_logger_thread_safe() {
    let config = LoggerConfig {
        max_string_len: 1024,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: "./logs/1".into(),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path.clone()).unwrap();
    let thread = logger.init(false);

    assert!(thread.is_ok());

    fs::remove_file(log_path).unwrap();
}

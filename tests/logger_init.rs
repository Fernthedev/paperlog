use std::{fs, time::Duration};

use paper2::types::config::LoggerConfig;

#[test]
fn test_logger_initialization() {
    let context_log_path = std::env::temp_dir().join("paper").join("test_init_10");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 1024,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path,
        log_path: log_path.clone(),
    };

    let _thread = config.init(false).unwrap();

    std::thread::sleep(Duration::from_millis(500));

    if cfg!(feature = "file") {
        assert!(log_path.exists());
        fs::remove_file(log_path).unwrap();
    }
}

#[test]
fn test_logger_config() {
    let context_log_path = std::env::temp_dir().join("paper").join("test_init_1");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 2048,
        log_max_buffer_count: 100,
        line_end: '\r',
        context_log_path,
        log_path,
    };

    assert_eq!(config.max_string_len, 2048);
    assert_eq!(config.log_max_buffer_count, 100);
    assert_eq!(config.line_end, '\r');
}

#[test]
fn test_logger_thread_safe() {
    let context_log_path = std::env::temp_dir().join("paper").join("test_init_1b");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 1024,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path,
        log_path: log_path.clone(),
    };

    let logger = config.init(false);

    assert!(logger.is_ok());

    fs::remove_file(log_path).unwrap();
}

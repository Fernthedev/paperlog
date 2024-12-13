#[test]
fn test_logger_initialization() {
    let config = LoggerConfig {
        max_string_len: 1024,
        log_max_buffer_count: 50,
        line_end: '\n',
    };

    let log_path = PathBuf::from("test_log.txt");
    let logger = LoggerThread::new(config, log_path.clone()).unwrap();
    let thread = logger.init().unwrap();

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
    };

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
    };

    let log_path = PathBuf::from("test_log_safe.txt");
    let logger = LoggerThread::new(config, log_path.clone()).unwrap();
    let thread = logger.init().unwrap();

    let thread_safe_logger = ThreadSafeLoggerThread::new(thread);
    assert!(thread_safe_logger.is_some());

    fs::remove_file(log_path).unwrap();
}

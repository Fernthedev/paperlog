#[test]
fn test_logger_thread_initialization() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
    };
    let log_path = PathBuf::from("test_log.log");

    let logger_thread = LoggerThread::new(config, log_path).unwrap();
    assert!(!logger_thread.inited.load(Ordering::SeqCst));
    assert!(logger_thread.thread_id.is_none());
}

#[test]
fn test_logger_thread_init() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
    };
    let log_path = PathBuf::from("test_log.log");

    let logger_thread = LoggerThread::new(config, log_path).unwrap();
    let thread_safe_logger = logger_thread.init().unwrap();

    let logger_thread = thread_safe_logger.read().unwrap();
    assert!(logger_thread.inited.load(Ordering::SeqCst));
    assert!(logger_thread.thread_id.is_some());
}

#[test]
fn test_queue_log() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
    };
    let log_path = PathBuf::from("test_log.log");

    let logger_thread = LoggerThread::new(config, log_path).unwrap();
    let thread_safe_logger = logger_thread.init().unwrap();

    let logger_thread = thread_safe_logger.read().unwrap();
    logger_thread.queue_log(LogLevel::Info, "test", "This is a test log");

    let (semaphore, queue) = logger_thread.log_queue.as_ref();
    let queue = queue.lock().unwrap();
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_add_sink() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
    };
    let log_path = PathBuf::from("test_log.log");

    let mut logger_thread = LoggerThread::new(config, log_path).unwrap();
    logger_thread.add_sink(|log_data| {
        println!("Log: {:?}", log_data);
    });

    assert_eq!(logger_thread.sinks.len(), 1);
}

#[test]
fn test_log_thread() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
    };
    let log_path = PathBuf::from("test_log.log");

    let logger_thread = LoggerThread::new(config, log_path).unwrap();
    let thread_safe_logger = logger_thread.init().unwrap();

    let (tx, rx) = channel();
    let logger_thread_clone = Arc::clone(&thread_safe_logger);

    thread::spawn(move || {
        let logger_thread = logger_thread_clone.read().unwrap();
        logger_thread.queue_log(LogLevel::Info, "test", "This is a test log");
        tx.send(()).unwrap();
    });

    rx.recv().unwrap();

    let logger_thread = thread_safe_logger.read().unwrap();
    let (semaphore, queue) = logger_thread.log_queue.as_ref();
    let queue = queue.lock().unwrap();
    assert_eq!(queue.len(), 1);
}

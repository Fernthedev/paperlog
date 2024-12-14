use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
    thread, time::Duration,
};


use crate::{log_level::LogLevel, LoggerConfig, LoggerThread};

#[test]
fn test_logger_thread_initialization() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/9"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger_thread = LoggerThread::new(config, log_path).unwrap();
    assert!(!logger_thread.is_inited().load(Ordering::SeqCst));
}

#[test]
fn test_logger_thread_init() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/8"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger_thread = LoggerThread::new(config, log_path).unwrap();
    let thread_safe_logger = logger_thread.init(false).unwrap();

    let logger_thread = thread_safe_logger.read().unwrap();
    assert!(logger_thread.is_inited().load(Ordering::SeqCst));
}

#[test]
fn test_queue_log() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: "./logs".into(),
    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger_thread = LoggerThread::new(config, log_path).unwrap();
    logger_thread.queue_log(
        LogLevel::Info,
        Some("test".to_string()),
        "This is a test log".to_string(),
        line!().to_string(),
        line!(),
    );

    {
        let queue = logger_thread.get_queue();
        let queue = queue.lock().unwrap();
        assert_eq!(queue.len(), 1);
    }

    {
        let thread_safe_logger = logger_thread.init(false).unwrap();
        thread::sleep(Duration::from_millis(500));

        thread_safe_logger.read().unwrap().wait_for_flush_timeout(Duration::from_millis(500));

        let logger_thread_read = thread_safe_logger.read().unwrap();
        let queue = logger_thread_read.get_queue();

        let queue = queue.lock().unwrap().len();
        assert_eq!(queue, 0);
    }
}

// #[test]
// fn test_add_sink() {
//     let config = LoggerConfig {
//         max_string_len: 100,
//         log_max_buffer_count: 50,
//         line_end: '\n',
//         context_log_path: "./logs".into(),
//     };
//     let log_path = PathBuf::from("./logs/test_log.log");

//     let mut logger_thread = LoggerThread::new(config, log_path).unwrap();
//     logger_thread.add_sink(|log_data: &LogData| -> Result<()> {
//         println!("Log: {:?}", log_data);
//         Ok(())
//     });

//     assert_eq!(logger_thread.get_sinks().len(), 1);
// }

#[test]
fn test_log_thread() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: "./logs".into(),
    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger_thread = LoggerThread::new(config, log_path).unwrap();
    let thread_safe_logger = logger_thread.init(false).unwrap();

    let logger_thread_clone = Arc::clone(&thread_safe_logger);

    thread::spawn(move || {
        let logger_thread = logger_thread_clone.read().unwrap();
        logger_thread.queue_log(
            LogLevel::Info,
            Some("test".to_string()),
            "This is a test log".to_string(),
            file!().to_string(),
            line!(),
        );
    })
    .join()
    .unwrap();

    let logger_thread = thread_safe_logger.read().unwrap();
    logger_thread.wait_for_flush_timeout(Duration::from_millis(500));
    let queue = logger_thread.get_queue().lock().unwrap().len();
    assert_eq!(queue, 0);
}

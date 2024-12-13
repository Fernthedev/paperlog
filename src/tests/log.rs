use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::sync::mpsc::channel;
use std::path::PathBuf;
use crate::logger::{Logger, LoggerConfig, LogLevel};

fn wait_for_complete_flush(logger: &Logger) {
    thread::sleep(Duration::from_millis(2));
    logger.wait_for_flush();
    while logger.log_queue_size() > 0 {
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn test_log_init() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs"),
    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger = Logger::new(config, log_path).unwrap();
    logger.wait_for_flush();
    assert!(logger.is_inited());
}

#[test]
fn test_log_output() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs"),

    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger = Logger::new(config, log_path).unwrap();
    let output = std::panic::catch_unwind(|| {
        logger.fmt_log(LogLevel::Info, "hi! {}", 5);
        wait_for_complete_flush(&logger);
    }).unwrap();

    assert_eq!(output, "Level (INFO) [GLOBAL] hi! 5\n");
}

#[test]
fn test_single_thread_log_spam() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs"),

    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger = Logger::new(config, log_path).unwrap();
    let output = std::panic::catch_unwind(|| {
        logger.fmt_log(LogLevel::Debug, "Spam logging now!");
        for i in 0..100000 {
            logger.fmt_log(LogLevel::Debug, "log i {}", i);
        }
        wait_for_complete_flush(&logger);
    }).unwrap();

    println!("Single thread took {}ms", output.elapsed().as_millis());
}

#[test]
fn test_multithread_log_spam() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs"),

    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger = Arc::new(Logger::new(config, log_path).unwrap());
    let output = std::panic::catch_unwind(|| {
        logger.fmt_log(LogLevel::Debug, "Spam logging now!");
        let mut handles = vec![];
        for _ in 0..4 {
            let logger = Arc::clone(&logger);
            handles.push(thread::spawn(move || {
                for i in 0..100000 {
                    logger.fmt_log(LogLevel::Debug, "log i {}", i);
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        wait_for_complete_flush(&logger);
    }).unwrap();

    println!("Multithread thread took {}ms", output.elapsed().as_millis());
}

#[test]
fn test_log_context_output() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs"),

    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger = Logger::new(config, log_path).unwrap();
    let context = logger.with_context("Context");
    let output = std::panic::catch_unwind(|| {
        context.fmt_log(LogLevel::Info, "context hi! {}", 6);
        wait_for_complete_flush(&logger);
    }).unwrap();

    assert_eq!(output, "Level (INFO) [Context] context hi! 6\n");
}

#[test]
fn test_log_context_tag_output() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs"),

    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger = Logger::new(config, log_path).unwrap();
    let context = "Context";
    let output = std::panic::catch_unwind(|| {
        logger.fmt_log_tag(LogLevel::Info, "hi this is a context log! {}", context, 5);
        wait_for_complete_flush(&logger);
    }).unwrap();

    assert_eq!(output, "Level (INFO) [Context] hi this is a context log! 5\n");
}

#[test]
fn test_utf8() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs"),

    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger = Logger::new(config, log_path).unwrap();
    let output = std::panic::catch_unwind(|| {
        logger.fmt_log(LogLevel::Info, "£ ह € 한");
        wait_for_complete_flush(&logger);
    }).unwrap();

    assert_eq!(output, "Level (INFO) [GLOBAL] £ ह € 한\n");
}

#[test]
fn test_utf16_to_utf8() {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs"),

    };
    let log_path = PathBuf::from("./logs/test_log.log");

    let logger = Logger::new(config, log_path).unwrap();
    let output = std::panic::catch_unwind(|| {
        logger.fmt_log(LogLevel::Info, "Testing UTF-16 conversion chars {}", String::from_utf16(&[0xD55C, 0x1F32E, 0x1F980]).unwrap());
        wait_for_complete_flush(&logger);
    }).unwrap();

    assert_eq!(output, "Level (INFO) [GLOBAL] Testing UTF-16 conversion chars 한🌮🦀\n");
}

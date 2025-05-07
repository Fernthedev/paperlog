use color_eyre::eyre::Result;

use crate::log_level::LogLevel;
use crate::logger::{LogData, LoggerConfig};
use crate::LoggerThread;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(feature = "tracing")]
use tracing_test::traced_test;

// TODO: Make these tests work with stdout

fn wait_for_complete_flush(logger: &LoggerThread) {
    thread::sleep(Duration::from_millis(2));
    // logger.wait_for_flush();
    logger.wait_for_flush_timeout(Duration::from_millis(500));
}

fn find_log(log_str: impl Into<String>) -> impl Fn(&[&str]) -> Result<(), String> {
    let log = log_str.into();

    // Ensure that the string `logged` is logged exactly twice
    move |lines: &[&str]| {
        let found = lines.iter().any(|line| line.contains(&log));

        match found {
            true => Ok(()),
            false => Err(format!(
                "Unable to find string {log} in logs. Logs: \n{}",
                lines.join("\n")
            )),
        }
    }
}

#[test]
fn test_log_init() -> Result<()> {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/1"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path).unwrap();
    assert!(!logger.is_inited().load(std::sync::atomic::Ordering::SeqCst));

    let logger = logger.init(false)?;
    thread::sleep(Duration::from_millis(2));

    assert!(logger
        .read()
        .unwrap()
        .is_inited()
        .load(std::sync::atomic::Ordering::SeqCst));
    Ok(())
}

#[cfg(feature = "tracing")]
#[test]
#[traced_test]
fn test_log_output() -> Result<()> {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/2"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.read().unwrap().queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: "hi! 5".to_owned(),
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger.read().unwrap());
    };

    logs_assert(find_log("Info [GLOBAL] hi! 5"));

    Ok(())
}

#[test]
fn test_single_thread_log_spam() -> Result<()> {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/3"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let output = {
        let start = Instant::now();
        logger.read().unwrap().queue_log(LogData {
            level: LogLevel::Debug,
            tag: None,
            message: "Spam logging now!".to_owned(),
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function_name: None,

            ..Default::default()
        });

        for i in 0..100000 {
            logger.read().unwrap().queue_log(LogData {
                level: LogLevel::Debug,
                tag: None,
                message: format!("log i {i}"),
                file: file!().to_string(),
                line: line!(),
                column: column!(),
                function_name: None,
                ..Default::default()
            });
        }
        wait_for_complete_flush(&logger.read().unwrap());
        start.elapsed()
    };

    println!("Single thread took {}ms", output.as_millis());

    Ok(())
}

#[test]
fn test_multithread_log_spam() -> Result<()> {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/4"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let output = {
        let start = Instant::now();
        logger.read().unwrap().queue_log(LogData {
            level: LogLevel::Debug,
            tag: None,
            message: "Spam logging now!".to_owned(),
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        let mut handles = vec![];
        for _ in 0..4 {
            let logger = Arc::clone(&logger);
            handles.push(thread::spawn(move || {
                for i in 0..100000 {
                    logger.read().unwrap().queue_log(LogData {
                        level: LogLevel::Debug,
                        tag: None,
                        message: format!("log i {i}"),
                        file: file!().to_string(),
                        line: line!(),
                        column: column!(),
                        function_name: None,
                        ..Default::default()
                    });
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        wait_for_complete_flush(&logger.read().unwrap());
        start.elapsed()
    };

    println!("Multithread thread took {}ms", output.as_millis());
    Ok(())
}

#[cfg(feature = "tracing")]
#[test]
#[traced_test]
fn test_log_context_output() -> Result<()> {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/5"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.read().unwrap().queue_log(LogData {
            level: LogLevel::Info,
            tag: Some("Context".to_string()),
            message: "context hi! 6".to_owned(),
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger.read().unwrap());
    };

    logs_assert(find_log("Info [Context] context hi! 6"));
    Ok(())
}

#[cfg(feature = "tracing")]
#[test]
#[traced_test]
fn test_log_context_tag_output() -> Result<()> {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/6"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let context = "Context";
    {
        logger.read().unwrap().queue_log(LogData {
            level: LogLevel::Info,
            tag: Some(context.to_string()),
            message: "hi this is a context log! 5".to_owned(),
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger.read().unwrap());
        thread::sleep(Duration::from_millis(2));
    };

    logs_assert(find_log("Info [Context] hi this is a context log! 5"));
    Ok(())
}

#[cfg(feature = "tracing")]
#[test]
#[traced_test]
fn test_utf8() -> Result<()> {
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        line_end: '\n',
        context_log_path: PathBuf::from("./logs/7"),
    };
    let log_path = config.context_log_path.join("test_log.log");

    let logger = LoggerThread::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.read().unwrap().queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: "£ ह € 한".to_owned(),
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger.read().unwrap());
    };

    logs_assert(find_log("Info [GLOBAL] £ ह € 한\n"));
    Ok(())
}

// #[test]
// fn test_utf16_to_utf8() -> Result<()> {
//     let config = LoggerConfig {
//         max_string_len: 100,
//         log_max_buffer_count: 50,
//         line_end: '\n',
//         context_log_path: PathBuf::from("./logs/"),
//     };
//     let log_path = PathBuf::from("./logs/test_log.log");

//     let logger = LoggerThread::new(config, log_path)?.init(false)?;
//     let output = capture_stdout(|| {
//         logger.read().unwrap().queue_log(
//             LogLevel::Info,
//             None,
//             format!(
//                 "Testing UTF-16 conversion chars {}",
//                 String::from_utf16(u"한🌮🦀").unwrap()
//             ),
//             file!().to_string(),
//             line!(),
//         );
//         wait_for_complete_flush(&logger.read().unwrap());
//     })?;

//     assert_eq!(
//         output,
//         "Info [GLOBAL] Testing UTF-16 conversion chars 한🌮🦀\n"
//     );
//     Ok(())
// }

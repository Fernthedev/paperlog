use color_eyre::eyre::Result;

use paper2::logger::LoggerThreadCtx;
use paper2::types::config::LoggerConfig;
use paper2::types::log_level::LogLevel;
use paper2::types::LogData;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(feature = "tracing")]
use tracing_test::traced_test;

// TODO: Make these tests work with stdout

fn wait_for_complete_flush(logger: &LoggerThreadCtx) {
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

    let logger = LoggerThreadCtx::new(config, log_path).unwrap();
    assert!(!logger.is_inited().load(std::sync::atomic::Ordering::SeqCst));

    let logger = logger.init(false)?;
    thread::sleep(Duration::from_millis(2));

    assert!(logger
        .read()
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

    let logger = LoggerThreadCtx::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.read().queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: "hi! 5".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger.read());
    };

    logs_assert(find_log("INFO [GLOBAL] hi! 5"));

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

    let logger = LoggerThreadCtx::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let output = {
        let start = Instant::now();
        logger.read().queue_log(LogData {
            level: LogLevel::Debug,
            tag: None,
            message: "Spam logging now!".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,

            ..Default::default()
        });

        for i in 0..100000 {
            logger.read().queue_log(LogData {
                level: LogLevel::Debug,
                tag: None,
                message: format!("log i {i}").into(),
                file: file!().into(),
                line: line!(),
                column: column!(),
                function_name: None,
                ..Default::default()
            });
        }
        wait_for_complete_flush(&logger.read());
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

    let logger = LoggerThreadCtx::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let output = {
        let start = Instant::now();
        logger.read().queue_log(LogData {
            level: LogLevel::Debug,
            tag: None,
            message: "Spam logging now!".into(),
            file: file!().into(),
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
                    logger.read().queue_log(LogData {
                        level: LogLevel::Debug,
                        tag: None,
                        message: format!("log i {i}").into(),
                        file: file!().into(),
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
        wait_for_complete_flush(&logger.read());
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

    let logger = LoggerThreadCtx::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.read().queue_log(LogData {
            level: LogLevel::Info,
            tag: Some("Context".into()),
            message: "context hi! 6".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger.read());
    };

    logs_assert(find_log("INFO [Context] context hi! 6"));
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

    let logger = LoggerThreadCtx::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let context = "Context";
    {
        logger.read().queue_log(LogData {
            level: LogLevel::Info,
            tag: Some(context.into()),
            message: "hi this is a context log! 5".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger.read());
        thread::sleep(Duration::from_millis(2));
    };

    logs_assert(find_log("INFO [Context] hi this is a context log! 5"));
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

    let logger = LoggerThreadCtx::new(config, log_path)?.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.read().queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: "£ ह € 한".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger.read());
    };

    logs_assert(find_log("INFO [GLOBAL] £ ह € 한\n"));
    Ok(())
}

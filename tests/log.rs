use color_eyre::eyre::Result;

use paper2::logger::Logger;
use paper2::types::config::LoggerConfig;
use paper2::types::log_level::LogLevel;
use paper2::types::LogData;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(feature = "tracing")]
use tracing_test::traced_test;

// TODO: Make these tests work with stdout

fn wait_for_complete_flush(logger: &Logger) {
    thread::sleep(Duration::from_millis(2));
    // logger.wait_for_flush();
    logger.wait_for_flush_timeout(Duration::from_millis(500));
}

/// Panics unless some captured `tracing` line contains every fragment in
/// `parts`. The line format is `LEVEL <time> [TAG] [file:line:col @ fn]
/// message`, so tests match the stable pieces rather than the whole line
/// (the time varies).
///
/// Reads `tracing-test`'s whole capture buffer instead of using the
/// injected `logs_assert`: that only sees lines recorded inside the test's
/// own span, and these events are emitted from paper2's logging thread,
/// which isn't in it. The messages checked are unique per test, so sharing
/// the buffer between tests is fine.
#[cfg(feature = "tracing")]
fn assert_logged(parts: &[&str]) {
    let buf = tracing_test::internal::global_buf().lock().unwrap();
    let logs = String::from_utf8_lossy(&buf);

    let found = logs
        .lines()
        .any(|line| parts.iter().all(|part| line.contains(part)));

    assert!(
        found,
        "Unable to find a line containing all of {parts:?} in logs. Logs:\n{logs}"
    );
}

#[test]
fn test_log_init() -> Result<()> {
    let context_log_path = std::env::temp_dir().join("paper").join("test_log_1");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        context_log_path,
        log_path,
        ..Default::default()
    };

    let _logger = config.init(false)?;
    thread::sleep(Duration::from_millis(2));

    Ok(())
}

#[cfg(feature = "tracing")]
#[test]
#[traced_test]
fn test_log_output() -> Result<()> {
    let context_log_path = std::env::temp_dir().join("paper").join("test_log_2");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        context_log_path,
        log_path,
        ..Default::default()
    };

    let logger = config.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: "hi! 5".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger);
    };

    assert_logged(&["INFO ", "[GLOBAL]", "] hi! 5"]);

    Ok(())
}

#[test]
fn test_single_thread_log_spam() -> Result<()> {
    let context_log_path = std::env::temp_dir().join("paper").join("test_log_3");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        context_log_path,
        log_path,
        ..Default::default()
    };

    let logger = config.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let output = {
        let start = Instant::now();
        logger.queue_log(LogData {
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
            logger.queue_log(LogData {
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
        wait_for_complete_flush(&logger);
        start.elapsed()
    };

    println!("Single thread took {}ms", output.as_millis());

    Ok(())
}

#[test]
fn test_multithread_log_spam() -> Result<()> {
    let context_log_path = std::env::temp_dir().join("paper").join("test_log_4");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        context_log_path,
        log_path,
        ..Default::default()
    };

    let logger = config.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let output = {
        let start = Instant::now();
        logger.queue_log(LogData {
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
            // `Logger` is cheap to clone (a channel `Sender` + one `Arc`
            // refcount bump), so no `Arc::clone` wrapper is needed here.
            let logger = logger.clone();
            handles.push(thread::spawn(move || {
                for i in 0..100000 {
                    logger.queue_log(LogData {
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
        wait_for_complete_flush(&logger);
        start.elapsed()
    };

    println!("Multithread thread took {}ms", output.as_millis());
    Ok(())
}

#[cfg(feature = "tracing")]
#[test]
#[traced_test]
fn test_log_context_output() -> Result<()> {
    let context_log_path = std::env::temp_dir().join("paper").join("test_log_5");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        context_log_path,
        log_path,
        ..Default::default()
    };

    let logger = config.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.queue_log(LogData {
            level: LogLevel::Info,
            tag: Some("Context".into()),
            message: "context hi! 6".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger);
    };

    assert_logged(&["INFO ", "[Context]", "] context hi! 6"]);
    Ok(())
}

#[cfg(feature = "tracing")]
#[test]
#[traced_test]
fn test_log_context_tag_output() -> Result<()> {
    let context_log_path = std::env::temp_dir().join("paper").join("test_log_6");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        context_log_path,
        log_path,
        ..Default::default()
    };

    let logger = config.init(false)?;
    thread::sleep(Duration::from_millis(2));

    let context = "Context";
    {
        logger.queue_log(LogData {
            level: LogLevel::Info,
            tag: Some(context.into()),
            message: "hi this is a context log! 5".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger);
        thread::sleep(Duration::from_millis(2));
    };

    assert_logged(&["INFO ", "[Context]", "] hi this is a context log! 5"]);
    Ok(())
}

#[cfg(feature = "tracing")]
#[test]
#[traced_test]
fn test_utf8() -> Result<()> {
    let context_log_path = std::env::temp_dir().join("paper").join("test_log_7");
    let log_path = context_log_path.join("test_log.log");
    let config = LoggerConfig {
        max_string_len: 100,
        log_max_buffer_count: 50,
        context_log_path,
        log_path,
        ..Default::default()
    };

    let logger = config.init(false)?;
    thread::sleep(Duration::from_millis(2));

    {
        logger.queue_log(LogData {
            level: LogLevel::Info,
            tag: None,
            message: "£ ह € 한".into(),
            file: file!().into(),
            line: line!(),
            column: column!(),
            function_name: None,
            ..Default::default()
        });
        wait_for_complete_flush(&logger);
    };

    assert_logged(&["INFO ", "[GLOBAL]", "] £ ह € 한"]);
    Ok(())
}

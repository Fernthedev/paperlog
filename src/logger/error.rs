//! Reporting for errors inside the logger itself (failing outputs on the
//! logging thread, locks unavailable in the panic hook). These can't go
//! through the logger, so they go to stderr, and to logcat on Android.

/// Where an internal logger error came from. Each source is tracked
/// separately, so e.g. a broken C sink doesn't hide a later disk-full error.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ErrorSource {
    File,
    Flush,
    Stdout,
    Sink,
    Logcat,
    Tracing,
}

impl ErrorSource {
    const ALL: [ErrorSource; 6] = [
        ErrorSource::File,
        ErrorSource::Flush,
        ErrorSource::Stdout,
        ErrorSource::Sink,
        ErrorSource::Logcat,
        ErrorSource::Tracing,
    ];
}

const ERROR_SOURCE_COUNT: usize = ErrorSource::ALL.len();

/// Reports logging-thread errors without letting them stop logging.
///
/// An output that keeps failing (disk full, closed pipe, a sink that always
/// errors) would otherwise print an error per batch, so each source reports
/// once when it starts failing and once when it recovers.
#[derive(Default)]
pub(crate) struct ErrorReporter {
    /// Source has reported an error and not yet recovered.
    failing: [bool; ERROR_SOURCE_COUNT],
    /// Source errored during the current batch.
    failed_this_batch: [bool; ERROR_SOURCE_COUNT],
}

impl ErrorReporter {
    pub(crate) fn check<E: std::fmt::Display>(
        &mut self,
        source: ErrorSource,
        result: std::result::Result<(), E>,
    ) {
        if let Err(e) = result {
            self.error(source, &e);
        }
    }

    pub(crate) fn error(&mut self, source: ErrorSource, error: &dyn std::fmt::Display) {
        let i = source as usize;
        self.failed_this_batch[i] = true;
        if !std::mem::replace(&mut self.failing[i], true) {
            report_internal(format_args!(
                "paper2: {source:?} output failed; further {source:?} errors are suppressed until it recovers: {error}"
            ));
        }
    }

    /// Ends a batch: any failing source that didn't error this batch has
    /// recovered.
    pub(crate) fn end_batch(&mut self) {
        for (i, failed) in self.failed_this_batch.iter_mut().enumerate() {
            if !std::mem::take(failed) && std::mem::take(&mut self.failing[i]) {
                report_internal(format_args!(
                    "paper2: {:?} output recovered",
                    ErrorSource::ALL[i]
                ));
            }
        }
    }
}

/// Last-resort reporting for errors inside the logger itself, which can't
/// go through the logger.
pub(crate) fn report_internal(args: std::fmt::Arguments<'_>) {
    eprintln!("{args}");

    // stderr goes nowhere on Android; mirror to logcat when it's available.
    #[cfg(all(target_os = "android", feature = "logcat"))]
    crate::logger::sinks::logcat_logger::log_error(args.to_string());
}

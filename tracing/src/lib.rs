use paper2::log_level::LogLevel;
use std::ffi::CString;
use std::{fmt, ptr};
use tracing::{Event, Metadata};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Default)]
struct MessageVisitor {
    parts: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if !self.parts.is_empty() {
            self.parts.push(' ');
        }
        self.parts
            .push_str(&format!("{}={:?}", field.name(), value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if !self.parts.is_empty() {
            self.parts.push(' ');
        }
        self.parts.push_str(&format!("{}={}", field.name(), value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if !self.parts.is_empty() {
            self.parts.push(' ');
        }
        self.parts.push_str(&format!("{}={}", field.name(), value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if !self.parts.is_empty() {
            self.parts.push(' ');
        }
        self.parts.push_str(&format!("{}={}", field.name(), value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if !self.parts.is_empty() {
            self.parts.push(' ');
        }
        self.parts.push_str(&format!("{}={}", field.name(), value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if !self.parts.is_empty() {
            self.parts.push(' ');
        }
        self.parts.push_str(&format!("{}={}", field.name(), value));
    }
}

/// Mapping from `tracing` levels to `paper2::log_level::LogLevel`.
fn map_level(meta: &Metadata<'_>) -> LogLevel {
    use tracing::Level;
    match *meta.level() {
        Level::ERROR => LogLevel::Error,
        Level::WARN => LogLevel::Warn,
        Level::INFO => LogLevel::Info,
        Level::DEBUG => LogLevel::Debug,
        Level::TRACE => LogLevel::Debug,
    }
}

/// A tracing subscriber layer that forwards events into the `paper2` logger.
pub struct PaperLayer {
    tag: Option<String>,
}

impl PaperLayer {
    pub fn new() -> Self {
        Self { tag: None }
    }

    pub fn with_tag<T: Into<String>>(mut self, tag: T) -> Self {
        self.tag = Some(tag.into());
        self
    }
}

impl<S> Layer<S> for PaperLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = map_level(meta);
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        // If the event didn't provide a `message` field, fall back to the target and fields string.
        let message = if !visitor.parts.is_empty() {
            visitor.parts
        } else {
            // Use the target as a minimal fallback
            meta.target().to_string()
        };

        let file = meta.file().map(|s| s.to_string()).unwrap_or_default();
        let line = meta.line().unwrap_or(0);
        let function_name = CString::new(meta.target()).expect("Function name");

        // use C ABI to call FFI logging function
        unsafe {
            let tag = self
                .tag
                .as_ref()
                .and_then(|s| std::ffi::CString::new(s.as_str()).ok());
            paper2::ffi::paper2_queue_log_ffi(
                level,
                tag.map(|c| c.as_ptr()).unwrap_or(ptr::null()),
                std::ffi::CString::new(message).unwrap().as_ptr(),
                std::ffi::CString::new(file).unwrap().as_ptr(),
                line as i32,
                0,
                function_name.as_ptr(),
            )
        };

        // if let Some(logger) = get_logger() {
        //     let _ = logger.read().unwrap().queue_log(LogData::new(
        //         level,
        //         self.tag.clone(),
        //         message,
        //         file,
        //         line,
        //         0,
        //         None,
        //     ));
        // }
    }
}

/// Installs the `PaperLayer` as a global subscriber (adds it to the registry).
///
/// Returns `Ok(())` if installed successfully, or the `tracing` error if a global
/// subscriber is already set.
pub fn init_paper_tracing() -> Result<(), tracing_subscriber::util::TryInitError> {
    use tracing_subscriber::registry::Registry;

    let layer = PaperLayer::new();
    let subscriber = Registry::default().with(layer);

    subscriber.try_init()
}

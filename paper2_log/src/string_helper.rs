//! String helpers for turning `log` records into the bytes paper2 queues.

use std::cell::RefCell;
use std::fmt;

thread_local! {
    /// Per-thread scratch buffer for formatting messages. paper2 copies the
    /// message when it's queued, so the formatted text only has to live for
    /// the FFI call and the buffer's allocation can be reused.
    static MESSAGE_BUF: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Don't let one huge message pin a huge buffer on the thread forever.
const MESSAGE_BUF_KEEP_CAPACITY: usize = 64 * 1024;

/// Calls `f` with the formatted message, without allocating when possible:
/// a plain literal (`info!("hello")`) is passed through as-is, and anything
/// else is formatted into the reusable thread-local buffer.
pub(crate) fn with_message<R>(args: &fmt::Arguments<'_>, f: impl FnOnce(&str) -> R) -> R {
    if let Some(literal) = args.as_str() {
        return f(literal);
    }

    let mut f = Some(f);
    let result = MESSAGE_BUF
        .try_with(|buf| {
            // Fails if formatting `args` itself logs (re-entrancy); fall
            // back to a one-off allocation in that case.
            let mut buf = buf.try_borrow_mut().ok()?;
            buf.clear();
            let _ = fmt::write(&mut *buf, *args);
            let result = (f.take()?)(&buf);
            if buf.capacity() > MESSAGE_BUF_KEEP_CAPACITY {
                buf.clear();
                buf.shrink_to(MESSAGE_BUF_KEEP_CAPACITY);
            }
            Some(result)
        })
        .ok()
        .flatten();

    match result {
        Some(result) => result,
        // Thread-local unavailable (thread teardown) or re-entered.
        None => (f.take().expect("`f` is only taken on success"))(&args.to_string()),
    }
}

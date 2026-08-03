#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
namespace Paper {
namespace ffi {
#endif  // __cplusplus

typedef enum paper2_LogLevel {
  Info,
  Warn,
  Error,
  Debug,
  Crit,
  Off,
} paper2_LogLevel;

/**
 * FFI-safe configuration for the logger.
 *
 * # Safety
 * - `context_log_path` must be a valid, null-terminated C string.
 * - All fields must be properly initialized before passing to FFI functions.
 */
typedef struct paper2_LoggerConfigFfi {
  unsigned long long max_string_len;
  unsigned long long log_max_buffer_count;
  unsigned char line_end;
  const char *context_log_path;
} paper2_LoggerConfigFfi;

/**
 * Helper struct to manage ownership of a byte slice across FFI boundaries.
 * I'm not sure if I can pass a struct using CString directly, so this is a workaround.
 *
 * Deliberately a raw `(ptr, len)` pair over arbitrary bytes rather than a
 * `&str` — the data ultimately comes from a caller-supplied C string that
 * isn't guaranteed to be valid UTF-8, and re-validating it here would mean
 * either rejecting or lossily rewriting bytes the caller explicitly wrote.
 */
typedef struct paper2_StringRef {
  const uint8_t *_0;
  uintptr_t _1;
} paper2_StringRef;

typedef struct paper2_LogDataC {
  enum paper2_LogLevel level;
  struct paper2_StringRef tag;
  struct paper2_StringRef message;
  int64_t timestamp;
  struct paper2_StringRef file;
  uint32_t line;
  uint32_t column;
  struct paper2_StringRef function_name;
} paper2_LogDataC;

/**
 * Extern "C" compatible log callback type.
 * The callback receives a pointer to LogData and a user-provided context pointer.
 * Returns 0 for success, nonzero for error.
 */
typedef void (*paper2_LogCallbackC)(const struct paper2_LogDataC *log_data, void *user_data);

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Initializes the logger from FFI.
 *
 * # Safety
 * - `config` and `path` must be valid pointers to initialized data.
 * - `path` must be a valid, null-terminated C string.
 * - This function should only be called once during initialization.
 */
bool paper2_init_logger_ffi(const struct paper2_LoggerConfigFfi *config, const char *path);

/**
 * Registers a new logging context by ID.
 *
 * # Safety
 * - `tag` must be a valid, null-terminated C string.
 */
void paper2_register_context_id(const char *tag);

/**
 * Unregisters a logging context by ID.
 *
 * # Safety
 * - `tag` must be a valid, null-terminated C string.
 */
void paper2_unregister_context_id(const char *tag);

/**
 * Queues a log entry from FFI.
 *
 * # Safety
 * - `tag`/`function_name` may be null (meaning "absent"); if non-null, and
 *   `message`/`file` always, must be valid, null-terminated C strings.
 * - `file` and `function_name` are stored as `&'static` references without
 *   copying — the pointed-to memory must remain valid for the rest of the
 *   process's lifetime. This holds for the common case (`__FILE__`/
 *   `__builtin_FUNCTION()`/`std::source_location`, all backed by static
 *   storage) but NOT for a transient/stack/heap buffer built per call; do
 *   not pass one of those here. `tag` and `message` are copied, so they
 *   have no such requirement.
 * - `level` must be a valid `LogLevel`.
 *
 * Prefer [`paper2_queue_log_bytes_ffi`] when the caller already knows each
 * string's length (e.g. `std::string_view::size()`) — it avoids a `strlen`
 * scan per field here, and doesn't require `tag`/`function_name` to be
 * null-terminated at all.
 */
bool paper2_queue_log_ffi(enum paper2_LogLevel level,
                          const char *tag,
                          const char *message,
                          const char *file,
                          int line,
                          int column,
                          const char *function_name);

/**
 * Queues a log entry from FFI using explicit `(pointer, length)` pairs
 * instead of null-terminated C strings.
 *
 * Prefer this over [`paper2_queue_log_ffi`] whenever the caller already has
 * a length on hand (`std::string_view`/`std::string` both do, for free).
 * Benefits over the C-string version:
 * - No `strlen` scan for any of the four fields.
 * - `tag`/`function_name` don't need to be null-terminated at all — a
 *   `std::string_view` that's a substring of a larger buffer is safe to
 *   pass directly, which it is *not* for [`paper2_queue_log_ffi`].
 * - Embedded NUL bytes in the middle of a string are preserved rather than
 *   silently truncating it, consistent with this crate's non-UTF-8-safe,
 *   arbitrary-byte-string design.
 *
 * # Safety
 * - `message_ptr`/`file_ptr` must be non-null and valid for reads of
 *   `message_len`/`file_len` bytes.
 * - `tag_ptr`/`function_name_ptr` may be null (meaning "absent"); if
 *   non-null, must be valid for reads of `tag_len`/`function_name_len`
 *   bytes.
 * - `file` and `function_name` are stored as `&'static` references without
 *   copying — see the equivalent note on [`paper2_queue_log_ffi`]; the
 *   same static-storage requirement applies here.
 * - `level` must be a valid `LogLevel`.
 */
bool paper2_queue_log_bytes_ffi(enum paper2_LogLevel level,
                                const uint8_t *tag_ptr,
                                uintptr_t tag_len,
                                const uint8_t *message_ptr,
                                uintptr_t message_len,
                                const uint8_t *file_ptr,
                                uintptr_t file_len,
                                int line,
                                int column,
                                const uint8_t *function_name_ptr,
                                uintptr_t function_name_len);

/**
 * Waits for all logs to be flushed.
 *
 * # Safety
 * - No arguments. Safe to call if logger is initialized.
 */
bool paper2_wait_for_flush(void);

/**
 * Gets the log directory as a C string.
 *
 * # Safety
 * - Returns a pointer to a heap-allocated C string. Caller must eventually free it.
 * - Safe to call if logger is initialized.
 */
const char *paper2_get_log_directory(void);

/**
 * Returns whether the logger is initialized.
 *
 * # Safety
 * - No arguments. Safe to call at any time.
 */
bool paper2_get_inited(void);

/**
 * Waits for all logs to be flushed, with a timeout in milliseconds.
 *
 * # Safety
 * - Safe to call if logger is initialized.
 */
bool paper2_wait_flush_timeout(unsigned int timeout_ms);

/**
 * Shuts down the logger, flushing all logs.
 * # Safety
 * - Safe to call if logger is initialized.
 */
bool paper2_add_log_sink(paper2_LogCallbackC callback, void *user_data);

/**
 * Frees a C string allocated by `paper2_get_log_directory`.
 * # Safety
 */
void paper2_free_c_string(char *s);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#ifdef __cplusplus
}  // namespace ffi
}  // namespace Paper
#endif  // __cplusplus

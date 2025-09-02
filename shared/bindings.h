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
 * Helper struct to manage ownership of C strings across FFI boundaries.
 * I'm not sure if I can pass a struct using CString directly, so this is a workaround.
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
 * - All pointer arguments must be valid, null-terminated C strings.
 * - `level` must be a valid `LogLevel`.
 */
bool paper2_queue_log_ffi(enum paper2_LogLevel level,
                          const char *tag,
                          const char *message,
                          const char *file,
                          int line,
                          int column,
                          const char *function_name);

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

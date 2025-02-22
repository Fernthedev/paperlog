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
} paper2_LogLevel;

typedef struct paper2_LoggerConfigFfi {
  unsigned long long max_string_len;
  unsigned long long log_max_buffer_count;
  unsigned char line_end;
  const char *context_log_path;
} paper2_LoggerConfigFfi;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

bool paper2_init_logger_ffi(const struct paper2_LoggerConfigFfi *config, const char *path);

void paper2_register_context_id(const char *tag);

void paper2_unregister_context_id(const char *tag);

bool paper2_queue_log_ffi(enum paper2_LogLevel level,
                          const char *tag,
                          const char *message,
                          const char *file,
                          int line,
                          int column,
                          const char *function_name);

bool paper2_wait_for_flush(void);

const char *paper2_get_log_directory(void);

bool paper2_get_inited(void);

bool paper2_wait_flush_timeout(unsigned int timeout_ms);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#ifdef __cplusplus
}  // namespace ffi
}  // namespace Paper
#endif  // __cplusplus

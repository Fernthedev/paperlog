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

typedef struct paper2_ThreadSafeLoggerThreadFfi {
  void *_0;
} paper2_ThreadSafeLoggerThreadFfi;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

struct paper2_ThreadSafeLoggerThreadFfi init_logger_ffi(const char *path);

void queue_log_ffi(struct paper2_ThreadSafeLoggerThreadFfi logger_thread,
                   enum paper2_LogLevel level,
                   const char *tag,
                   const char *message,
                   const char *file,
                   int line);

bool wait_for_flush(struct paper2_ThreadSafeLoggerThreadFfi logger_thread);

bool get_inited(struct paper2_ThreadSafeLoggerThreadFfi logger_thread);

bool wait_flush_timeout(struct paper2_ThreadSafeLoggerThreadFfi logger_thread, int timeout_ms);

void free_logger_thread(struct paper2_ThreadSafeLoggerThreadFfi logger_thread);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#ifdef __cplusplus
}  // namespace ffi
}  // namespace Paper
#endif  // __cplusplus

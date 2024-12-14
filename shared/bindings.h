#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

enum class LogLevel {
  Info,
  Warn,
  Error,
  Debug,
};

struct ThreadSafeLoggerThreadFfi {
  void *_0;
};

extern "C" {

ThreadSafeLoggerThreadFfi init_logger_ffi(const char *path);

void queue_log_ffi(ThreadSafeLoggerThreadFfi logger_thread,
                   LogLevel level,
                   const char *tag,
                   const char *message,
                   const char *file,
                   int line);

bool wait_for_flush(ThreadSafeLoggerThreadFfi logger_thread);

bool get_inited(ThreadSafeLoggerThreadFfi logger_thread);

bool wait_flush_timeout(ThreadSafeLoggerThreadFfi logger_thread, int timeout_ms);

void free_logger_thread(ThreadSafeLoggerThreadFfi logger_thread);

}  // extern "C"

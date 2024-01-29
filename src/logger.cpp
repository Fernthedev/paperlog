#include <fmt/ostream.h>
#include <chrono>
#include <fmt/chrono.h>
#include <fmt/compile.h>
#include <fmt/std.h>

#include <exception>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <ratio>
#include <semaphore>
#include <span>
#include <thread>
#include <unordered_map>
#include <vector>

#include "internal_logger.hpp"
#include "logger.hpp"
#include "early.hpp"
#include "internal.hpp"
#include "log_level.hpp"
#include "queue/blockingconcurrentqueue.h"
#include "queue/concurrentqueue.h"

#include <csignal>

#if __has_include(<unwind.h>)
#include <cxxabi.h>
#include <dlfcn.h>
#include <unistd.h>
#include <unwind.h>

#define HAS_UNWIND
#else
#warning No unwind support, backtraces will do nothing
#endif

#ifdef PAPERLOG_FMT_NO_PREFIX
#warning Removing fmt prefixes from logs
#endif

#ifdef PAPERLOG_STDOUT_LOG
#warning Logging to stdout!
#endif

#ifdef PAPERLOG_ANDROID_LOG
#warning Logging to android logcat!
#include <android/log.h>
#endif

#ifdef PAPERLOG_GLOBAL_FILE_LOG
#warning Logging to file log! Does not fully comply at the moment
#endif

#ifdef PAPERLOG_CONTEXT_FILE_LOG
#warning Logging to context file log! Does not fully comply at the moment
#endif

// extern defines
EARLY_INIT_ATTRIBUTE PAPER_EXPORT moodycamel::BlockingConcurrentQueue<Paper::LogData> Paper::Internal::logQueue;

#pragma region internals

// fields
namespace {
EARLY_INIT_ATTRIBUTE std::binary_semaphore flushSemaphore{ 1 };
EARLY_INIT_ATTRIBUTE static Paper::LoggerConfig globalLoggerConfig;
EARLY_INIT_ATTRIBUTE static std::filesystem::path globalLogPath;

EARLY_INIT_ATTRIBUTE static std::vector<Paper::LogSink> sinks;
EARLY_INIT_ATTRIBUTE static std::unordered_map<ContextID, LogPath, StringHash, std::equal_to<>> registeredFileContexts;

EARLY_INIT_ATTRIBUTE static LogPath globalFile;

// To avoid loading errors
volatile bool inited = false;
std::optional<std::thread::id> threadId;
} // namespace
namespace {
namespace Sinks {

void fileSink(Paper::LogData const& threadData, std::string_view fmtMessage, std::string_view unformattedMessage,
              /* nullable */ std::ofstream* contextFilePtr) {
#ifdef PAPERLOG_GLOBAL_FILE_LOG
  globalFile << fmtMessage << '\n';
#endif
#ifdef PAPERLOG_CONTEXT_FILE_LOG
  if (contextFilePtr != nullptr) {
    auto& f = *contextFilePtr;
    f << fmtMessage << '\n';
  }
#endif
}

void stdOutSink(Paper::LogData const& threadData, std::string_view fmtMessage, std::string_view _unformattedMessage) {
  auto const& level = threadData.level;
  auto const& tag = threadData.tag;

  std::cout << "Level (" << Paper::format_as(level) << ") ";
  std::cout << "[" << tag << "] ";
  std::cout << fmtMessage << std::endl;
}

#ifdef PAPERLOG_ANDROID_LOG
void androidLogcatSink(Paper::LogData const& threadData, std::string_view _fmtMessage,
                       std::string_view unformattedMessage) {
  auto const& level = threadData.level;
  auto const& tag = threadData.tag;

  // Reduce log bloat for android logcat
#ifdef PAPERLOG_FMT_NO_PREFIX
  auto androidMsg = unformattedMessage;
#else
  auto const& location = threadData.loc;
  auto const& threadId = fmt::to_string(threadData.threadId);

  // TODO: Reduce double formatting
  std::string_view locationFileName(location.file_name());
  auto maxLen = std::min<size_t>(locationFileName.size() - globalLoggerConfig.MaximumFileLengthInLogcat, 0);

  // limit length
  // don't allow file name to be super long
  locationFileName = locationFileName.substr(maxLen);

  std::string androidMsg(fmt::format(FMT_COMPILE("{}[{:<6}] [{}:{}:{} @ {}]: {}"), level, threadId, locationFileName,
                                     location.line(), location.column(), location.function_name(), unformattedMessage));

#endif
  __android_log_write(static_cast<int>(level), tag.data(), androidMsg.data());
}
#endif
} // namespace Sinks
} // namespace

namespace {
void logInternal(Paper::LogLevel level, std::string_view s) {
#ifdef PAPERLOG_ANDROID_LOG
  __android_log_write(static_cast<int>(level), "PAPER", s.data());
#endif

#ifdef PAPERLOG_STDOUT_LOG
  std::cout << "PAPER: [" << Paper::format_as(level) << "] " << s << std::endl;
#endif
}

void logError(std::string_view error) {
  logInternal(Paper::LogLevel::ERR, error);
  if (globalFile.is_open()) {
    globalFile << "PAPER INTERNAL ERROR " << error << std::endl;
    globalFile.flush();
  }
}

[[nodiscard]] constexpr uint8_t charExtraLength(char const c) {
  uint8_t shiftedC = c >> 3;

  if (shiftedC >= 0b11110) {
    return 3;
  }
  if (shiftedC >= 0b11100) {
    return 2;
  }

  if (shiftedC >= 0b11000) {
    return 1;
  }

  return 0;
}

inline void writeLog(Paper::LogData const& threadData, std::tm const& time, std::string_view threadId,
                     std::string_view originalString,
                     /* nullable */ std::ofstream* contextFilePtr) {

  auto const& tag = threadData.tag;
  auto const& location = threadData.loc;
  auto const& level = threadData.level;

  // "{Ymd} [{HMSf}] {l}[{t:<6}] [{s}]"
#ifndef PAPERLOG_FMT_NO_PREFIX
  std::string const fullMessage(fmt::format(FMT_COMPILE("{:%Y-%m-%d} [{:%H:%M:%S}] {}[{:<6}] [{}] [{}:{}:{} @ {}]: {}"),
                                            time, time, level, threadId, tag, location.file_name(), location.line(),
                                            location.column(), location.function_name(),
                                            originalString // TODO: Is there a better way to do this?
                                            ));
#else
  std::string_view const fullMessage(originalString);
#endif

  // realtime android logging
#ifdef PAPERLOG_ANDROID_LOG
  Sinks::androidLogcatSink(threadData, fullMessage, originalString);
#endif
  // realtime console logging
#ifdef PAPERLOG_STDOUT_LOG
  Sinks::stdOutSink(threadData, fullMessage, originalString);
#endif

  // file logging
  Sinks::fileSink(threadData, fullMessage, originalString, contextFilePtr);

  // additional sinks
  for (auto const& sink : sinks) {
    sink(threadData, fullMessage, originalString);
  }
}
} // namespace

namespace {
void signal_handler(int signal) {
  logInternal(Paper::LogLevel::ERR, fmt::format("Received signal handler {}, waiting to flush!", signal));
  if (!inited) {
    return;
  }
  if (std::this_thread::get_id() == threadId) {
    logInternal(Paper::LogLevel::ERR, "Signal was called from log thread!");
    return;
  }

  Paper::Logger::WaitForFlush();
  while (Paper::Internal::logQueue.size_approx() > 0 && inited) {
    std::this_thread::sleep_for(std::chrono::microseconds(100));
  }
}
} // namespace

#pragma endregion

#pragma region LoggerImpl

PAPER_EXPORT void Paper::Logger::Init(std::string_view logPath) {
  LoggerConfig config{};
  return Init(logPath, config);
}

PAPER_EXPORT void Paper::Logger::Init(std::string_view logPath, LoggerConfig const& config) {
  if (inited) {
    return;
    // throw std::runtime_error("Already started the logger thread!");
  }

  logInternal(Paper::LogLevel::INF, "Logging paper to folder " + std::string(logPath) + "and file " + GLOBAL_FILE_NAME);

  globalLoggerConfig = { config };
  globalLogPath = logPath;
  std::filesystem::create_directories(globalLogPath);

  auto globalFileFilePath = std::filesystem::path(logPath) / GLOBAL_FILE_NAME;

  globalFile.open(globalFileFilePath, std::ofstream::out | std::ofstream::trunc);
  std::thread(Internal::LogThread).detach();
  flushSemaphore.release();
}

PAPER_EXPORT bool Paper::Logger::IsInited() {
  return inited;
}

#pragma endregion

#pragma region Internal

PAPER_EXPORT void Paper::Internal::Queue(Paper::LogData&& threadData) noexcept {
  Internal::logQueue.enqueue(std::forward<Paper::LogData>(threadData));
}

PAPER_EXPORT void Paper::Internal::Queue(Paper::LogData&& threadData, moodycamel::ProducerToken const& token) noexcept {
  Internal::logQueue.enqueue(token, std::forward<Paper::LogData>(threadData));
}
PAPER_EXPORT moodycamel::ProducerToken Paper::Internal::MakeProducerToken() noexcept {
  return moodycamel::ProducerToken(logQueue);
}

PAPER_EXPORT void Paper::Internal::LogThread() {
  try {
    inited = true;
    threadId = std::optional(std::this_thread::get_id());
    
    std::signal(SIGINT, signal_handler);
    std::signal(SIGABRT, signal_handler);
    std::signal(SIGFPE, signal_handler);
    std::signal(SIGILL, signal_handler);
    std::signal(SIGSEGV, signal_handler);
    std::signal(SIGTERM, signal_handler);

    moodycamel::ConsumerToken token(Paper::Internal::logQueue);

    auto constexpr logBulkCount = 50;
    std::array<Paper::LogData, logBulkCount> threadQueue;

    std::ofstream* contextFile = nullptr;
    std::string_view selectedContext;

    size_t logsSinceLastFlush = 0;
    std::chrono::system_clock::time_point lastLogTime = std::chrono::system_clock::now();

    bool doFlush = false;
    auto flushLambda = [&]() {
      // nothing more in queue, flush
      globalFile.flush();
      for (auto& context : registeredFileContexts) {
        if (context.second.is_open()) {
          context.second.flush();
        }
      }

      logsSinceLastFlush = 0;
      doFlush = false;
      lastLogTime = std::chrono::system_clock::now();

      flushSemaphore.release();
    };

    // Write to log to empty
    flushLambda();

    while (true) {
      size_t dequeCount = 0;

      // Wait a while for new logs to show
      if (doFlush) {
        dequeCount = Paper::Internal::logQueue.wait_dequeue_bulk_timed(token, threadQueue.data(), logBulkCount,
                                                                       std::chrono::milliseconds(10));
      } else {
        // wait indefinitely for new logs since we don't need to flush
        dequeCount = Paper::Internal::logQueue.wait_dequeue_bulk(token, threadQueue.data(), logBulkCount);
      }

      // Check if we should flush
      if (dequeCount == 0) {
        if (doFlush) {
          flushLambda();
        }
        flushSemaphore.release();
        std::this_thread::yield();
        std::this_thread::sleep_for(std::chrono::microseconds(400));
        continue;
      }

      for (size_t i = 0; i < dequeCount; i++) {
        auto const& threadData = threadQueue[i];
        auto const& rawFmtStr = threadData.str;
        auto const& tag = threadData.tag;
        auto const& location = threadData.loc;
        auto const& level = threadData.level;
        auto const& systemTime = std::chrono::system_clock::to_time_t(threadData.logTime);
        auto const& time = fmt::localtime(systemTime);
        std::string threadId = fmt::to_string(threadData.threadId);

        if (tag != selectedContext) {
          if (tag.empty()) {
            contextFile = nullptr;
          } else {
            // std::unique_lock lock(contextMutex);
            auto it = registeredFileContexts.find(tag);
            if (it != registeredFileContexts.end()) {
              contextFile = &it->second;
            }
          }

          selectedContext = tag;
        }

        auto writeLogLambda = [&](std::string_view view) constexpr {
          writeLog(threadData, time, threadId, view, contextFile);
          doFlush = true;
        };

        // Split/chunk string algorithm provided by sc2ad thanks
        // intended for logcat and making \n play nicely
        auto maxStrLength = std::min<size_t>(rawFmtStr.size(), globalLoggerConfig.MaxStringLen);
        auto const* begin = rawFmtStr.data();
        std::size_t stringEndOffset = 0;
        uint8_t skipCount = 0;

        for (auto const& c : rawFmtStr) {
          if (skipCount > 0) {
            skipCount--;
            stringEndOffset++;
            continue;
          }

          // line break, write and continue
          if (c == globalLoggerConfig.lineEnd) {
            //  TODO: string view length not being respected in Clang 15
            //  Linux when writing to std::cout
            // causing line break to not work
            writeLogLambda(std::string_view(begin, stringEndOffset));
            begin += stringEndOffset + 1;
            stringEndOffset = 0;
            // skipping extra bytes because utf8 is variable
          } else if ((skipCount = charExtraLength(c)) > 0) {
            stringEndOffset++;
            // string reached max length
          } else if (stringEndOffset >= maxStrLength) {
            writeLogLambda(std::string_view(begin, stringEndOffset));
            begin += stringEndOffset;
            stringEndOffset = 1;
            // increment string end index
          } else {
            stringEndOffset++;
          }
        }
        // Write remaining string contents
        if (stringEndOffset > 0) {
          writeLogLambda(std::string_view(begin, stringEndOffset));
        }
        logsSinceLastFlush++;

        // Since I completely forgot what happened here
        // This commit suggests it reduces latency
        // https://github.com/Fernthedev/paperlog/commit/931b15a7f5b494272b486acabc3062038db79fa1#diff-2c46dd80094c3ffd00cd309628cb1d6e5c695f69f8dafb5c40747369a5d6ded0R199

        // And also log if time has passed
        auto elapsedTime = std::chrono::system_clock::now() - lastLogTime;
        if (logsSinceLastFlush > globalLoggerConfig.LogMaxBufferCount || elapsedTime > std::chrono::seconds(1)) {
          flushLambda();
        }
      }
    }
  } catch (std::exception const& e) {
    std::string error = fmt::format("Error occurred in logging thread! {}", e.what());
    logError(error);
    inited = false;
    throw e;
  } catch (...) {
    std::string error = fmt::format("Error occurred in logging thread!");
    logError(error);
    inited = false;
    throw;
  }

  logInternal(Paper::LogLevel::INF, "Finished log thread");
}

PAPER_EXPORT void Paper::Logger::WaitForFlush() {
  flushSemaphore.acquire();
}

PAPER_EXPORT std::filesystem::path Paper::Logger::getLogDirectoryPathGlobal() {
  return globalLogPath;
}

// TODO: Lock?
PAPER_EXPORT void Paper::Logger::RegisterFileContextId(std::string_view contextId, std::string_view logPath) {

  auto filePath = getLogDirectoryPathGlobal() / logPath;
  filePath.replace_extension(".log");

  Paper::Logger::fmtLog<LogLevel::INF>("Registering context {} at path {}", contextId, filePath);

  std::ofstream f;
  f.open(filePath, std::ofstream::out | std::ofstream::trunc);
  if (!f.is_open()) {
    Paper::Logger::fmtLog<LogLevel::INF>("Unable to register context {} at path {}", contextId, filePath);
    return;
  }
  registeredFileContexts.try_emplace(contextId.data(), std::move(f));
}

PAPER_EXPORT void Paper::Logger::UnregisterFileContextId(std::string_view contextId) {
  registeredFileContexts.erase(contextId.data());
}

PAPER_EXPORT void Paper::Logger::AddLogSink(LogSink const& sink) {
  sinks.emplace_back(sink);
}

PAPER_EXPORT Paper::LoggerConfig& Paper::Logger::GlobalConfig() {
  return globalLoggerConfig;
}

#pragma endregion

#pragma region BacktraceImpl
#ifdef HAS_UNWIND
// Backtrace stuff largely written by StackDoubleFlow
// https://github.com/sc2ad/beatsaber-hook/blob/138101a5a2b494911583b62140af6acf6e955e72/src/utils/logging.cpp#L211-L289
namespace {
struct BacktraceState {
  void** current;
  void** end;
};
static _Unwind_Reason_Code unwindCallback(struct _Unwind_Context* context, void* arg) {
  BacktraceState* state = static_cast<BacktraceState*>(arg);
  uintptr_t pc = _Unwind_GetIP(context);
  if (pc != 0U) {
    if (state->current == state->end) {
      return _URC_END_OF_STACK;
    } else {
      *state->current++ = reinterpret_cast<void*>(pc);
    }
  }
  return _URC_NO_REASON;
}
size_t captureBacktrace(void** buffer, uint16_t max) {
  BacktraceState state{ buffer, buffer + max };
  _Unwind_Backtrace(unwindCallback, &state);

  return state.current - buffer;
}
} // namespace

PAPER_EXPORT void Paper::Logger::Backtrace(std::string_view const tag, uint16_t frameCount) {
  void* buffer[frameCount + 1];
  captureBacktrace(buffer, frameCount + 1);
  fmtLogTag<LogLevel::DBG>("Printing backtrace with: {} max lines:", tag, frameCount);
  fmtLogTag<LogLevel::DBG>("*** *** *** *** *** *** *** *** *** *** *** *** *** *** *** ***", tag);
  fmtLogTag<LogLevel::DBG>("pid: {}, tid: {}", tag, getpid(), gettid());
  for (uint16_t i = 0; i < frameCount; ++i) {
    Dl_info info;
    if (dladdr(buffer[i + 1], &info) != 0) {
      // Buffer points to 1 instruction ahead
      long addr = reinterpret_cast<char*>(buffer[i + 1]) - reinterpret_cast<char*>(info.dli_fbase) - 4;
      if (info.dli_sname != nullptr) {
        int status;
        char const* demangled = abi::__cxa_demangle(info.dli_sname, nullptr, nullptr, &status);
        if (status != 0) {
          demangled = info.dli_sname;
        }
        fmtLogTag<LogLevel::DBG>("        #{:02}  pc {:016Lx}  {} ({})", tag, i, addr, info.dli_fname, demangled);
        if (demangled != info.dli_sname) {
          free(const_cast<char*>(demangled));
        }
      } else {
        fmtLogTag<LogLevel::DBG>("        #{:02}  pc {:016Lx}  {}", tag, i, addr, info.dli_fname);
      }
    }
  }
}

#else

#warning No unwind found, compiling stub backtrace function
PAPER_EXPORT void Paper::Logger::Backtrace(std::string_view const tag, uint16_t frameCount) {}

#endif

#pragma endregion

#include <chrono>
#include <fmt/chrono.h>
#include <fmt/compile.h>
#include <fmt/ostream.h>
#include <fmt/std.h>

#include <exception>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <semaphore>
#include <span>
#include <thread>
#include <unordered_map>
#include <vector>

#include "android_stubs.hpp"
#include "logger.hpp"
#include "early.hpp"
#include "internal.hpp"
#include "log_level.hpp"
#include "queue/blockingconcurrentqueue.h"
#include "queue/concurrentqueue.h"

#if __has_include(<unwind.h>)
#include <cxxabi.h>
#include <dlfcn.h>
#include <unistd.h>
#include <unwind.h>

#define HAS_UNWIND
#endif



EARLY_INIT_ATTRIBUTE moodycamel::BlockingConcurrentQueue<Paper::ThreadData> Paper::Internal::logQueue;
EARLY_INIT_ATTRIBUTE std::binary_semaphore flushSemaphore{ 1 };
EARLY_INIT_ATTRIBUTE static Paper::LoggerConfig globalLoggerConfig;
EARLY_INIT_ATTRIBUTE static std::filesystem::path globalLogPath;

EARLY_INIT_ATTRIBUTE static std::vector<Paper::LogSink> sinks;
EARLY_INIT_ATTRIBUTE static std::unordered_map<ContextID, LogPath, StringHash, std::equal_to<>> registeredFileContexts;

EARLY_INIT_ATTRIBUTE static LogPath globalFile;

#pragma region internals

namespace {
// To avoid loading errors
static bool inited = false;

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

inline void WriteStdOut(int level, std::string_view ctx, std::string_view s) {
#ifdef PAPERLOG_ANDROID_LOG
  __android_log_write(level, ctx.data(), s.data());
#endif

#ifdef PAPERLOG_FMT_C_STDOUT
#warning Printing to stdout
#ifndef PAPERLOG_FMT_NO_PREFIX
  fmt::print(std::cout, "Level ({}) [{}] ", level, ctx);
#endif
  std::cout << s << std::endl;
  // fmt::println(std::cout, fmt::runtime(s));
#endif
}

inline void logError(std::string_view error) {
  WriteStdOut(static_cast<int>(Paper::LogLevel::ERR), "PAPERLOG", error);
  if (globalFile.is_open()) {
    globalFile << error << std::endl;
    globalFile.flush();
  }
}

inline void writeLog(Paper::ThreadData const& threadData, std::tm const& time, std::string_view threadId,
                     std::string_view s,
                     /* nullable */ std::ofstream* contextFilePtr) {

  auto const& rawFmtStr = threadData.str;
  auto const& tag = threadData.tag;
  auto const& location = threadData.loc;
  auto const& level = threadData.level;

  // "{Ymd} [{HMSf}] {l}[{t:<6}] [{s}]"
#ifndef PAPERLOG_FMT_NO_PREFIX
  std::string const msg(fmt::format(FMT_COMPILE("{:%Y-%m-%d} [{:%H:%M:%S}] {}[{:<6}] [{}] [{}:{}:{} @ {}]: {}"), time,
                                    time, level, threadId, tag, location.file_name(), location.line(),
                                    location.column(), location.function_name(),
                                    s // TODO: Is there a better way to do this?
                                    ));

  // TODO: Reduce double formatting
  std::string_view locationFileName(location.file_name());

  // limit length
  locationFileName =
      locationFileName
          // don't allow file name to be super long
          .substr(std::min<size_t>(locationFileName.size() - globalLoggerConfig.MaximumFileLengthInLogcat, 0));

  std::string androidMsg(fmt::format(FMT_COMPILE("{}[{:<6}] [{}:{}:{} @ {}]: {}"), level, threadId, locationFileName,
                                     location.line(), location.column(), location.function_name(), s));
#else
  std::string const msg(s);
  std::string const androidMsg(s);
#endif

  WriteStdOut(static_cast<int>(level), tag, s.data());
  globalFile << msg << '\n';

  if (contextFilePtr != nullptr) {
    auto& f = *contextFilePtr;
    f << msg << '\n';
  }

  for (auto const& sink : sinks) {
    sink(threadData, msg);
  }
}
} // namespace

#pragma endregion

#pragma region LoggerImpl

namespace Paper::Logger {
void Init(std::string_view logPath, LoggerConfig const& config) {
  if (inited) {
    throw std::runtime_error("Already started the logger thread!");
  }

  WriteStdOut(ANDROID_LOG_INFO, "PAPERLOG",
              "Logging paper to folder " + std::string(logPath) + "and file " + GLOBAL_FILE_NAME);

  globalLoggerConfig = { config };
  globalLogPath = logPath;
  std::filesystem::create_directories(globalLogPath);

  auto globalFileFilePath = std::filesystem::path(logPath) / GLOBAL_FILE_NAME;

  globalFile.open(globalFileFilePath, std::ofstream::out | std::ofstream::trunc);
  std::thread(Internal::LogThread).detach();
  flushSemaphore.release();
  inited = true;
}

bool IsInited() {
  return inited;
}

} // namespace Paper::Logger

#pragma endregion

#pragma region Internal

// TODO: Fix constructor memory crash
#ifdef PAPER_NO_INIT
#warning Using dlopen for initializing thread
void __attribute__((constructor(1000))) dlopen_initialize() {
  WriteStdOut(ANDROID_LOG_INFO, "PAPERLOG", "DLOpen initializing");

#ifdef PAPER_QUEST_MODLOADER
  std::string path = fmt::format("/sdcard/Android/data/{}/files/logs/paper", Modloader::getApplicationId());
#elif defined(PAPER_QUEST_SCOTLAND2)
  std::string path = fmt::format("/sdcard/Android/data/{}/files/logs/paper", modloader::get_application_id());
#else
#warning "Must have a definition for globalLogPath if PAPER_NO_INIT is defined!
  std::string path(globalLogPath);
#endif
  try {
    Paper::Logger::Init(path, Paper::LoggerConfig());
  } catch (std::exception const& e) {
    std::string error = fmt::format("Error occurred in logging thread! {}", e.what());
    WriteStdOut(ANDROID_LOG_ERROR, "PAPERLOG", error);
    throw e;
  } catch (...) {
    std::string error = fmt::format("Error occurred in logging thread!");
    WriteStdOut(ANDROID_LOG_ERROR, "PAPERLOG", error);
    throw;
  }
}
#endif

void Paper::Internal::LogThread() {
  try {
    moodycamel::ConsumerToken token(Paper::Internal::logQueue);

    auto constexpr logBulkCount = 50;
    std::array<Paper::ThreadData, logBulkCount> threadQueue;

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
#ifndef PAPER_NO_INIT
    inited = false;
#endif
    throw e;
  } catch (...) {
    std::string error = fmt::format("Error occurred in logging thread!");
    logError(error);
#ifndef PAPER_NO_INIT
    inited = false;
#endif
    throw;
  }

  WriteStdOut(ANDROID_LOG_INFO, "PaperInternals", "Finished log thread");
}

void Paper::Logger::WaitForFlush() {
  flushSemaphore.acquire();
}

std::filesystem::path Paper::Logger::getLogDirectoryPathGlobal() {
  return globalLogPath;
}

// TODO: Lock?
void Paper::Logger::RegisterFileContextId(std::string_view contextId, std::string_view logPath) {

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

void Paper::Logger::UnregisterFileContextId(std::string_view contextId) {
  registeredFileContexts.erase(contextId.data());
}

void Paper::Logger::AddLogSink(LogSink const& sink) {
  sinks.emplace_back(sink);
}

Paper::LoggerConfig& Paper::Logger::GlobalConfig() {
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

void Paper::Logger::Backtrace(std::string_view const tag, uint16_t frameCount) {
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
void Paper::Logger::Backtrace(std::string_view const tag, uint16_t frameCount) {}

#endif

#pragma endregion
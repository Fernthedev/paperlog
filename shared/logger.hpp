#pragma once

/// DEFINE PAPER_INLINE_QUEUE to use slightly faster queue logic at the cost of
/// bigger binary size DEFINE NO_MODLOADER_FORMAT to disable modloader include
/// and fmt struct DEFINE NO_SL2_FORMAT to disable scotland2 include and fmt
/// struct

#include "_config.h"
#include "bindings.h"
#include "log_level.hpp"
#include <chrono>
#include <cstdint>
#include <fmt/base.h>
#include <fmt/xchar.h>
#include <thread>
#include <type_traits>

#ifdef PAPER_QUEST_MODLOADER
#include "feature/modinfo_fmt.hpp"
#endif
#ifdef PAPER_QUEST_SCOTLAND2
#include "feature/scotland2_fmt.hpp"
#endif

#include <filesystem>
#include <functional>
#include <optional>
#include <utility>

// TODO: Breaking change use std::source_location
#include "source_location.hpp"

namespace Paper {
/// High-level logging API for Paper.
///
/// The logger supports formatted messages, source-location capture, contextual
/// tags, and optional custom sinks. The free functions in Paper::Logger queue
/// messages into the runtime backend, while context helpers attach a tag for
/// contextual logging.
#ifndef NOSTD_SOURCE_LOCATION_HPP
using sl = PAPERLOG_SL_T;
#else
// #warning Using nostd source location
using sl = nostd::source_location;
#endif

enum class LogLevel : uint8_t;

using TimePoint = std::chrono::system_clock::time_point;
static constexpr std::string_view const GLOBAL_TAG = "GLOBAL";

#ifdef PAPER_ROOT_FOLDER_LENGTH
inline constexpr size_t SOURCE_OFFSET = PAPER_ROOT_FOLDER_LENGTH;
#else
inline constexpr size_t SOURCE_OFFSET = 0;
#endif

// TODO: Inherit when NDK fixes bug
// https://github.com/android/ndk/issues/1677
template <typename Char, typename... TArgs> struct BasicFmtStrSrcLoc {
  using ParentType = fmt::basic_format_string<Char, TArgs...>;
  ParentType parentType;

  explicit(false) inline operator ParentType&() {
    return parentType;
  }

  explicit(false) inline operator ParentType const&() const {
    return parentType;
  }

  explicit(false) inline operator fmt::string_view() {
    return parentType;
  }

  explicit(false) inline operator fmt::string_view const() const {
    return parentType;
  }

  sl sourceLocation;

  template <typename S>
    requires(std::is_convertible_v<S const&, fmt::basic_string_view<char>>)
  consteval inline BasicFmtStrSrcLoc(S const& s, sl const& sourceL = sl::current())
      : parentType(s), sourceLocation(sourceL) {}

  BasicFmtStrSrcLoc(fmt::runtime_format_string<char> r, sl const& sourceL = sl::current())
      : parentType(r), sourceLocation(sourceL) {}
};

template <typename... Args> using FmtStrSrcLoc = BasicFmtStrSrcLoc<char, std::type_identity_t<Args>...>;

/// Immutable view of a single log entry as delivered to sinks.
///
/// This mirrors the data emitted by the FFI layer and is passed to custom log
/// sink callbacks so downstream consumers can inspect the log level, message,
/// tag, and source metadata without parsing the raw C ABI payload.
struct LogData {
  LogLevel level;
  std::optional<std::string_view> tag;
  std::string_view message;
  std::string_view file;
  uint32_t line;
  uint32_t column;
  std::optional<std::string_view> function_name;
  int64_t time;

  LogData(Paper::ffi::paper2_LogDataC const& data)
      : level((LogLevel)data.level), line(data.line), column(data.column), time(data.timestamp) {
    message = std::string_view((char const*)data.message._0, data.message._1);
    file = std::string_view((char const*)data.file._0, data.file._1);

    if (data.tag._0 != nullptr && data.tag._1 > 0) {
      tag = std::string_view((char const*)data.tag._0, data.tag._1);
    }

    if (data.function_name._0 != nullptr && data.function_name._1 > 0) {
      function_name = std::string_view((char const*)data.function_name._0, data.function_name._1);
    }
  }
};

/// Signature for custom sink callbacks.
///
/// Each sink receives a fully populated log entry after formatting and metadata
/// extraction. This is the main extension point for routing logs to custom
/// backends or additional storage.
using LogSink = std::function<void(Paper::LogData const& logData)>;

/// Runtime configuration for the logger backend.
///
/// These values control buffering behavior, the maximum size of strings before
/// they are split, and how much of a message is preserved when routed to
/// logcat.
struct LoggerConfig {
  LoggerConfig() = default;

  char lineEnd = '\n';
  /**
   * @brief When the buffer exceeds this number with logs
   * pending to be written, it will force a flush
   *
   */
  uint16_t LogMaxBufferCount = 512;

  /**
   * @brief Maximum length for a string
   * When a string reaches/exceeds this length, it will be split
   *
   */
  uint32_t MaxStringLen = 1024;

  uint32_t MaximumFileLengthInLogcat = 50;
};

namespace Logger {
/// Queue a formatted log message with an explicit source location.
///
/// This is the lowest-level entry point used by the high-level helpers. It
/// renders the format string with fmt, captures the current source location,
/// and forwards the payload to the FFI logger backend.
inline void vfmtLog(fmt::string_view const str, LogLevel level, sl const& sourceLoc, std::string_view const tag,
                    fmt::format_args&& args) noexcept {
  auto message = fmt::vformat(str, args);

  Paper::ffi::paper2_queue_log_ffi((ffi::paper2_LogLevel)level, tag.data(), message.c_str(),
                                   sourceLoc.file_name().data() + size_t(SOURCE_OFFSET), sourceLoc.line(),
                                   sourceLoc.column(), sourceLoc.function_name().data());
}

/// Log a formatted message with an explicit tag.
///
/// The tag is used for contextual routing and file association. This overload
/// is the primary entry point when a log should be associated with a specific
/// context rather than the global logger.
template <LogLevel lvl, typename... TArgs>
constexpr auto fmtLogTag(FmtStrSrcLoc<TArgs...> str, std::string_view const tag, TArgs&&... args) {
  return Logger::vfmtLog(str, lvl, str.sourceLocation, tag, fmt::make_format_args(args...));
}

/// Log a formatted message using the global/default context.
///
/// This is the convenience overload for emitting logs without an explicit tag.
template <LogLevel lvl, typename... TArgs> constexpr auto fmtLog(FmtStrSrcLoc<TArgs...> str, TArgs&&... args) {
  return fmtLogTag<lvl, TArgs...>(str, {}, std::forward<TArgs>(args)...);
}

#ifdef __EXCEPTIONS
template <typename Exception = std::runtime_error, typename... TArgs>
inline void fmtThrowError(FmtStrSrcLoc<TArgs...> str, TArgs&&... args) {
  Logger::fmtLog<LogLevel::ERR, TArgs...>(std::forward(str), std::forward<TArgs>(args)...);
  throw Exception(fmt::vformat(str, fmt::make_format_args(args...)));
}

template <typename Exception = std::runtime_error, typename... TArgs>
inline void fmtThrowErrorTag(FmtStrSrcLoc<TArgs...> str, std::string_view const tag, TArgs&&... args) {
  Logger::fmtLogTag<LogLevel::ERR, TArgs...>(str, tag, std::forward<TArgs>(args)...);

  auto message = fmt::vformat(str, fmt::make_format_args(args...));
  throw Exception(fmt::format("{} {}", tag, message));
}
#endif

/// Return the directory used for Paper log files.
inline std::filesystem::path getLogDirectoryPathGlobal() {
  return Paper::ffi::paper2_get_log_directory();
}

/// Initialize the logger with a global log file path.
inline void Init(std::string_view globalLogFile) {
  Paper::ffi::paper2_init_logger_ffi(nullptr, globalLogFile.data());
}
/// Initialize the logger with a custom configuration and log path.
inline void Init(std::string_view logPath, LoggerConfig const& config) {
  ffi::paper2_LoggerConfigFfi configFfi = { config.MaxStringLen, config.LogMaxBufferCount, config.lineEnd, nullptr };
  Paper::ffi::paper2_init_logger_ffi(&configFfi, logPath.data());
}
/// Return whether the logger backend has been initialized.
inline bool IsInited() {
  return Paper::ffi::paper2_get_inited();
}

/// Register a context ID so logs for that context are written to a file.
inline void RegisterFileContextId(std::string_view contextId) {
  Paper::ffi::paper2_register_context_id(contextId.data());
}

/// Remove a previously registered file context ID.
inline void UnregisterFileContextId(std::string_view contextId) {
  Paper::ffi::paper2_unregister_context_id(contextId.data());
}

/// Block until the logger has flushed at least one pending log entry.
inline void WaitForFlush() {
  Paper::ffi::paper2_wait_for_flush();
}
/// Block up to the provided timeout in milliseconds for a flush to complete.
inline void WaitForFlushTimeout(uint32_t timeout) {
  Paper::ffi::paper2_wait_flush_timeout(timeout);
}

// defined in backtrace.hpp
void Backtrace(std::string_view const tag, uint16_t frameCount);

/// Register a custom sink that will receive every queued log entry.
inline void AddLogSink(LogSink tempSink) {

  Paper::ffi::paper2_add_log_sink(
      +[](Paper::ffi::paper2_LogDataC const* logData, void* userData) {
        LogSink& sink = *reinterpret_cast<LogSink*>(userData);
        return sink(Paper::LogData(*logData));
      },
      new LogSink(tempSink));
}
}; // namespace Logger

namespace Logger {
template <typename... TArgs> inline auto debug(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) {
  return Logger::fmtLog<LogLevel::DBG, TArgs...>(s, std::forward<TArgs>(args)...);
}
template <typename... TArgs> inline auto info(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) {
  return Logger::fmtLog<LogLevel::INF, TArgs...>(s, std::forward<TArgs>(args)...);
}
template <typename... TArgs> inline auto warn(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) {
  return Logger::fmtLog<LogLevel::WRN, TArgs...>(s, std::forward<TArgs>(args)...);
}
template <typename... TArgs> inline auto error(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) {
  return Logger::fmtLog<LogLevel::ERR, TArgs...>(s, std::forward<TArgs>(args)...);
}
template <typename... TArgs> inline auto critical(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) {
  return Logger::fmtLog<LogLevel::CRIT, TArgs...>(s, std::forward<TArgs>(args)...);
}
} // namespace Logger

/// Base type for context-aware logging helpers.
///
/// A logger context carries a tag that is automatically supplied to emitted
/// logs, allowing callers to group messages by subsystem or feature.
template <typename Str> struct BaseLoggerContext {
  Str tag;

  constexpr BaseLoggerContext(Str tag) : tag(std::move(tag)) {}
  constexpr BaseLoggerContext() noexcept = default;
  constexpr BaseLoggerContext(BaseLoggerContext&&) noexcept = default;
  constexpr BaseLoggerContext(BaseLoggerContext const&) noexcept = default;
  constexpr ~BaseLoggerContext() = default;

  BaseLoggerContext& operator=(BaseLoggerContext&& o) noexcept = default;
  BaseLoggerContext& operator=(BaseLoggerContext const& o) noexcept = default;

  template <LogLevel lvl, typename... TArgs> constexpr auto fmtLog(FmtStrSrcLoc<TArgs...> str, TArgs&&... args) const {
    return Logger::fmtLogTag<lvl, TArgs...>(str, tag, std::forward<TArgs>(args)...);
  }

  template <typename Exception = std::runtime_error, typename... TArgs>
  inline auto fmtThrowError(FmtStrSrcLoc<TArgs...> str, TArgs&&... args) const {
    return Logger::fmtThrowErrorTag<Exception, TArgs...>(str, tag, std::forward<TArgs>(args)...);
  }

  [[nodiscard]] inline auto Backtrace(uint16_t frameCount) const {
    return Logger::Backtrace(tag, frameCount);
  }

  template <typename... TArgs> inline auto debug(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) const {
    return this->fmtLog<LogLevel::DBG, TArgs...>(s, std::forward<TArgs>(args)...);
  }
  template <typename... TArgs> inline auto info(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) const {
    return this->fmtLog<LogLevel::INF>(s, std::forward<TArgs>(args)...);
  }
  template <typename... TArgs> inline auto warn(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) const {
    return this->fmtLog<LogLevel::WRN>(s, std::forward<TArgs>(args)...);
  }
  template <typename... TArgs> inline auto error(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) const {
    return this->fmtLog<LogLevel::ERR>(s, std::forward<TArgs>(args)...);
  }
  template <typename... TArgs> inline auto critical(FmtStrSrcLoc<TArgs...> const& s, TArgs&&... args) const {
    return this->fmtLog<LogLevel::CRIT>(s, std::forward<TArgs>(args)...);
  }
};

template <std::size_t sz> struct ConstLoggerContext : public BaseLoggerContext<char[sz]> {
  constexpr ConstLoggerContext(char const (&s)[sz]) : BaseLoggerContext<char[sz]>() {
    std::copy(s, s + sz, BaseLoggerContext<char[sz]>::tag);
  }
};

struct LoggerContext : public BaseLoggerContext<std::string> {
  explicit LoggerContext(std::string_view s) : BaseLoggerContext<std::string>(std::string(s)) {}

  // allow implicit conversion
  template <typename U>
    requires(std::is_constructible_v<std::string, U>)
  LoggerContext(BaseLoggerContext<U> const& s) : BaseLoggerContext<std::string>(s.tag) {}
};

namespace Logger {
/// Create a logger context from a compile-time string literal.
///
/// When enabled, the context ID is registered for file logging as well.
template <ConstLoggerContext ctx, bool registerFile = true> inline auto WithContext() {
  if constexpr (registerFile) {
    RegisterFileContextId(ctx.tag);
  }
  return ctx;
}
/// Create a logger context from a runtime string tag.
///
/// When enabled, the supplied tag is registered for file logging as well.
template <bool registerFile = true> inline auto WithContextRuntime(std::string_view const tag) {
  if constexpr (registerFile) {
    RegisterFileContextId(tag);
  }
  return LoggerContext(tag);
}
} // namespace Logger
} // namespace Paper

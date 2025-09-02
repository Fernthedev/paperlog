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
#ifndef NOSTD_SOURCE_LOCATION_HPP
using sl = PAPERLOG_SL_T;
#else
// #warning Using nostd source location
using sl = nostd::source_location;
#endif

enum class LogLevel : uint8_t;

using TimePoint = std::chrono::system_clock::time_point;
static constexpr std::string_view const GLOBAL_TAG = "GLOBAL";

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
      : level((LogLevel)data.level), message(data.message._0, data.message._1), file(data.file._0, data.file._1),
        line(data.line), column(data.column), time(data.timestamp) {
    if (data.tag._0 != nullptr && data.tag._1 > 0) {
      tag = std::string_view(data.tag._0, data.tag._1);
    }

    if (data.function_name._0 != nullptr && data.function_name._1 > 0) {
      function_name = std::string_view(data.function_name._0, data.function_name._1);
    }
  }
};

///
/// @param originalString This param exists since strings can be splitted due to
/// newlines etc. Use this when you need the exact printed string for this sink
/// call. Unformatted refers to no Paper prefixes. This does not give the
/// origianl string without the initial fmt run
using LogSink = std::function<void(Paper::Logger::LogData const& logData)>;

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
inline void vfmtLog(fmt::string_view const str, LogLevel level, sl const& sourceLoc, std::string_view const tag,
                    fmt::format_args&& args) noexcept {
  auto message = fmt::vformat(str, args);

  Paper::ffi::paper2_queue_log_ffi((ffi::paper2_LogLevel)level, tag.data(), message.c_str(),
                                   sourceLoc.file_name().data() + size_t(PAPER_ROOT_FOLDER_LENGTH), sourceLoc.line(),
                                   sourceLoc.column(), sourceLoc.function_name().data());
}

template <LogLevel lvl, typename... TArgs>
constexpr auto fmtLogTag(FmtStrSrcLoc<TArgs...> str, std::string_view const tag, TArgs&&... args) {
  return Logger::vfmtLog(str, lvl, str.sourceLocation, tag, fmt::make_format_args(args...));
}

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

inline std::filesystem::path getLogDirectoryPathGlobal() {
  return Paper::ffi::paper2_get_log_directory();
}

inline void Init(std::string_view logPath) {
  Paper::ffi::paper2_init_logger_ffi(nullptr, logPath.data());
}
inline void Init(std::string_view logPath, LoggerConfig const& config) {
  ffi::paper2_LoggerConfigFfi configFfi = { config.MaxStringLen, config.LogMaxBufferCount, config.lineEnd, nullptr };
  Paper::ffi::paper2_init_logger_ffi(&configFfi, logPath.data());
}
inline bool IsInited() {
  return Paper::ffi::paper2_get_inited();
}

inline void RegisterFileContextId(std::string_view contextId) {
  Paper::ffi::paper2_register_context_id(contextId.data());
}

inline void UnregisterFileContextId(std::string_view contextId) {
  Paper::ffi::paper2_unregister_context_id(contextId.data());
}

// blocks indefinitely until a log gets flushed
inline void WaitForFlush() {
  Paper::ffi::paper2_wait_for_flush();
}
inline void WaitForFlushTimeout(uint32_t timeout) {
  Paper::ffi::paper2_wait_flush_timeout(timeout);
}

// defined in backtrace.hpp
void Backtrace(std::string_view const tag, uint16_t frameCount);

// TODO: Sinks
inline void AddLogSink(LogSink sink) {
  Paper::ffi::paper2_add_log_sink(+[sink](Paper::ffi::paper2_LogDataC logData) { sink(Paper::LogData(logData)); }, nullptr)
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
template <ConstLoggerContext ctx, bool registerFile = true> inline auto WithContext() {
  if constexpr (registerFile) {
    RegisterFileContextId(ctx.tag);
  }
  return ctx;
}
template <bool registerFile = true> inline auto WithContextRuntime(std::string_view const tag) {
  if constexpr (registerFile) {
    RegisterFileContextId(tag);
  }
  return LoggerContext(tag);
}
} // namespace Logger
} // namespace Paper

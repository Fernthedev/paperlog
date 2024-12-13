#pragma once

/// DEFINE PAPER_INLINE_QUEUE to use slightly faster queue logic at the cost of bigger binary size
/// DEFINE NO_MODLOADER_FORMAT to disable modloader include and fmt struct
/// DEFINE NO_SL2_FORMAT to disable scotland2 include and fmt struct

#include "_config.h"
#include <chrono>
#include <fmt/base.h>
#include <fmt/xchar.h>
#include <thread>
#include <type_traits>
#include "log_level.hpp"
#include "internal_logger.hpp"

#ifdef PAPER_QUEST_MODLOADER
#include "feature/modinfo_fmt.hpp"
#endif
#ifdef PAPER_QUEST_SCOTLAND2
#include "feature/scotland2_fmt.hpp"
#endif

#include <functional>
#include <optional>
#include <filesystem>
#include <utility>

// TODO: Breaking change use std::source_location
#include "source_location.hpp"

#include "log_level.hpp"

// #include <fmtlog/fmtlog.h>

namespace Paper {
#ifndef NOSTD_SOURCE_LOCATION_HPP
using sl = PAPERLOG_SL_T;
#else
// #warning Using nostd source location
using sl = nostd::source_location;
#endif

enum class LogLevel : uint8_t;

using TimePoint = std::chrono::system_clock::time_point;

static constexpr const std::string_view GLOBAL_TAG = "GLOBAL";
//
//    template <typename... TArgs>
//    struct FmtStringHackTArgs {
//        fmt::format_string<TArgs...> str;
//        sl loc;
//
//        constexpr FmtStringHackTArgs(fmt::format_string<TArgs...> const& str, const sl& loc = sl::current()) :
//        str(str), loc(loc) {}
//    };
//
//    struct FmtStringHack {
//        fmt::string_view str;
//        sl loc;
//
//        template <typename... TArgs>
//        constexpr FmtStringHack(FmtStringHackTArgs<TArgs...> const& str) : FmtStringHack(str.str, str.loc) {}
//
//        constexpr FmtStringHack(const char* str, const sl& loc = sl::current())
//                : str(str), loc(loc) {}
//
//        constexpr FmtStringHack(std::string_view str, const sl& loc = sl::current())
//                : str(str), loc(loc) {}
//    };

//    template<class... TArgs>
//    struct BasicFmtStrSrcLoc : fmt::format_string<TArgs...> {
//        sl sourceLocation;
//        template <typename S>
//        requires (std::is_convertible_v<const S&, fmt::basic_string_view<char>>)
//        consteval inline BasicFmtStrSrcLoc(const S& s, sl sourceL = sl::current()) : fmt::format_string<TArgs...>(s),
//        sourceLocation(sourceL) {}
//    };

// TODO: Inherit when NDK fixes bug
// https://github.com/android/ndk/issues/1677
template <typename Char, typename... TArgs>
struct BasicFmtStrSrcLoc {
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

//    template <typename... Args>
//    using FmtStrSrcLoc = BasicFmtStrSrcLoc<char, fmt::type_identity_t<Args>...>;
template <typename... Args> using FmtStrSrcLoc = BasicFmtStrSrcLoc<char, std::type_identity_t<Args>...>;

///
/// @param originalString This param exists since strings can be splitted due to newlines etc.
/// Use this when you need the exact printed string for this sink call.
/// Unformatted refers to no Paper prefixes.
/// This does not give the origianl string without the initial fmt run
using LogSink = std::function<void(LogData const&, std::string_view fmtMessage, std::string_view originalString)>;

struct PAPER_EXPORT LoggerConfig {
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
#ifdef PAPER_INLINE_QUEUE
  Internal::logQueue.enqueue(Paper::ThreadData(fmt::vformat(str, args), std::this_thread::get_id(), tag, sourceLoc,
                                               level, std::chrono::system_clock::now()));
#else
  ::Paper::Internal::Queue(Paper::LogData(fmt::vformat(str, args), std::this_thread::get_id(), tag, sourceLoc, level,
                                          std::chrono::system_clock::now()));
#endif
}

template <LogLevel lvl, typename... TArgs>
constexpr auto fmtLogTag(FmtStrSrcLoc<TArgs...> str, std::string_view const tag, TArgs&&... args) {
  return Logger::vfmtLog(str, lvl, str.sourceLocation, tag, fmt::make_format_args(args...));
}

template <LogLevel lvl, typename... TArgs> constexpr auto fmtLog(FmtStrSrcLoc<TArgs...> str, TArgs&&... args) {
  return fmtLogTag<lvl, TArgs...>(str, GLOBAL_TAG, std::forward<TArgs>(args)...);
}

template <typename Exception = std::runtime_error, typename... TArgs>
inline void fmtThrowError(FmtStrSrcLoc<TArgs...> const& str, TArgs&&... args) {
  Logger::fmtLog<LogLevel::ERR, TArgs...>(str, std::forward<TArgs>(args)...);
  throw Exception(fmt::format<TArgs...>(str, std::forward<TArgs>(args)...));
}

template <typename Exception = std::runtime_error, typename... TArgs>
inline void fmtThrowErrorTag(FmtStrSrcLoc<TArgs...> const& str, std::string_view const tag, TArgs&&... args) {
  Logger::fmtLogTag<LogLevel::ERR, TArgs...>(str, tag, std::forward<TArgs>(args)...);

  auto exceptionMsg = fmt::format<TArgs...>(str, std::forward<TArgs>(args)...);
  throw Exception(fmt::format("{} {}", tag, exceptionMsg));
}

PAPER_EXPORT void Backtrace(std::string_view tag, uint16_t frameCount);

inline auto Backtrace(uint16_t frameCount) {
  return Backtrace(GLOBAL_TAG, frameCount);
}

PAPER_EXPORT std::filesystem::path getLogDirectoryPathGlobal();

PAPER_EXPORT void Init(std::string_view logPath);
PAPER_EXPORT void Init(std::string_view logPath, LoggerConfig const& config);
PAPER_EXPORT bool IsInited();

PAPER_EXPORT void RegisterFileContextId(std::string_view contextId, std::string_view logPath);

inline auto RegisterFileContextId(std::string_view contextId) {
  return Logger::RegisterFileContextId(contextId, contextId);
}

PAPER_EXPORT void UnregisterFileContextId(std::string_view contextId);

PAPER_EXPORT void WaitForFlush();
/**
 * @brief Returns a mutable reference to the global configuration.
 * NOTE THAT MODIFYING THIS MAY NOT BE UPDATED ON THE CURRENT FLUSH DUE TO RACE CONDITIONS!
 *
 * @return LoggerConfig& The mutable reference to the global configuration.
 **/
PAPER_EXPORT LoggerConfig& GlobalConfig();

PAPER_EXPORT void AddLogSink(LogSink const& sink);
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

  template <LogLevel lvl, typename... TArgs>
  constexpr auto fmtLog(FmtStrSrcLoc<TArgs...> const& str, TArgs&&... args) const {
    return Logger::fmtLogTag<lvl, TArgs...>(str, tag, std::forward<TArgs>(args)...);
  }

  template <typename Exception = std::runtime_error, typename... TArgs>
  inline auto fmtThrowError(FmtStrSrcLoc<TArgs...> const& str, TArgs&&... args) const {
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

template <std::size_t sz>
struct ConstLoggerContext : public BaseLoggerContext<char[sz]> {
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
template <ConstLoggerContext ctx, bool registerFile = true>
inline auto WithContext(std::string_view const logFile = ctx.tag) {
  if constexpr (registerFile) {
    RegisterFileContextId(ctx.tag, logFile);
  }
  return ctx;
}
template <bool registerFile = true>
inline auto WithContextRuntime(std::string_view const tag, std::optional<std::string_view> logFile = {}) {
  if constexpr (registerFile) {
    RegisterFileContextId(tag, logFile.value_or(tag));
  }
  return LoggerContext(tag);
}
} // namespace Logger
} // namespace Paper
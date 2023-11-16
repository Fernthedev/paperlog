#pragma once

#include <utility>
#include "queue/blockingconcurrentqueue.h"
#include "queue/concurrentqueue.h"

// TODO: Breaking change use std::source_location
#include "source_location.hpp"

#include "log_level.hpp"

// #if __has_include(<source_location>)
// #include <source_location>
// #define PAPERLOG_SL_T std::source_location
// #elif __has_include(<experimental/source_location>)
// #define PAPERLOG_SL_T std::experimental::source_location
// #include <experimental/source_location>
// #elif defined(PAPERLOG_SL_T)
// #error
// #else
// #include "source_location.hpp"
// #endif

// // NDK R26 is weird
// #if !__has_builtin(__builtin_source_location)
// #include "source_location.hpp"
// #define NOSTD_SOURCE_LOCATION_HPP
// #endif

namespace Paper {
#ifndef NOSTD_SOURCE_LOCATION_HPP
using sl = PAPERLOG_SL_T;
#else
// #warning Using nostd source location
using sl = nostd::source_location;
#endif

enum class LogLevel : uint8_t;

using TimePoint = std::chrono::system_clock::time_point;

struct ThreadData {
  ThreadData(ThreadData const&) = delete;
  ThreadData(ThreadData&&) = default;

  explicit ThreadData() : loc(Paper::sl::current()){};

  ThreadData(std::string str, std::thread::id threadId, std::string_view tag, sl const& loc,
             LogLevel level, TimePoint logTime)
      : str(std::move(str)), threadId(threadId), tag(tag), loc(loc), level(level), logTime(logTime) {}

  ~ThreadData() = default;

  //        constexpr ThreadData(fmt::string_view const &str, fmt::format_args const &args, std::thread::id const
  //        &threadId,
  //                             std::string_view const &tag, sl const &loc, LogLevel level,
  //                             TimePoint const &logTime)
  //                : str(str), args(args), threadId(threadId), tag(tag), loc(loc), level(level), logTime(logTime) {}

  //        ThreadData& operator =(ThreadData&& o) noexcept {
  //            str = o.str;
  //            args = o.args;
  //            threadId = o.threadId;
  //            tag = o.tag;
  //            loc = std::move(o.loc);
  //            level = o.level;
  //            logTime = o.logTime;
  //            return *this;
  //        };

  ThreadData& operator=(ThreadData&& o) noexcept = default;
  ThreadData& operator=(ThreadData const& o) noexcept = delete;

  std::string str;
  //        fmt::string_view str;
  //        fmt::format_args args;
  std::thread::id threadId;
  std::string tag; // TODO: Use std::string_view?
  sl loc;
  LogLevel level = LogLevel::INF;
  TimePoint logTime;
};

namespace Internal {
void LogThread();

void Queue(ThreadData&& threadData) noexcept;
void Queue(ThreadData&& threadData, moodycamel::ProducerToken const& token) noexcept;
moodycamel::ProducerToken MakeProducerToken() noexcept;

// [[deprecated("Do not call! Kept for legacy purposes")]]
extern moodycamel::BlockingConcurrentQueue<ThreadData> logQueue;
} // namespace Internal
} // namespace Paper
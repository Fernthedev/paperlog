#pragma once

#if __has_include(<source_location>)
#include <source_location>
#elif __has_include(<experimental/source_location>)
#include <experimental/source_location>
#else
#include <utility>

#include "source_location.hpp"
#endif

#include "queue/concurrentqueue.h"

namespace Paper {
#ifndef NOSTD_SOURCE_LOCATION_HPP
    using sl = std::experimental::source_location;
#else
    using sl = nostd::source_location;
#endif

    enum class LogLevel : uint8_t;

    using TimePoint = std::chrono::system_clock::time_point;

    struct ThreadData {
        ThreadData(ThreadData const&) = delete;
        ThreadData(ThreadData&&) = default;

        ThreadData(std::string str, std::thread::id const &threadId,
                             std::string_view const &tag, sl const &loc, LogLevel level,
                             TimePoint const &logTime)
                : str(std::move(str)), threadId(threadId), tag(tag), loc(loc), level(level), logTime(logTime) {}

        ThreadData(std::string_view const str, std::thread::id const &threadId,
                   std::string_view const &tag, sl const &loc, LogLevel level,
                   TimePoint const &logTime)
                : str(str), threadId(threadId), tag(tag), loc(loc), level(level), logTime(logTime) {}

//        constexpr ThreadData(fmt::string_view const &str, fmt::format_args const &args, std::thread::id const &threadId,
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

        ThreadData& operator =(ThreadData&& o) noexcept = default;

        std::string str;
//        fmt::string_view str;
//        fmt::format_args args;
        std::thread::id threadId;
        std::string tag; // TODO: Use std::string_view?
        sl loc;
        LogLevel level;
        TimePoint logTime;
    };

    namespace Internal {
        void LogThread();

        extern moodycamel::ConcurrentQueue<ThreadData> logQueue;
    }
}
#pragma once

#include <thread>
#include <fstream>
#include <iostream>
#include <string_view>
#include <fmt/core.h>

//#include <fmtlog/fmtlog.h>

namespace fmtlog {
    enum class LogLevel : uint8_t
    {
        DBG = 0,
        INF,
        WRN,
        ERR,
        OFF
    };
}

#if __has_include(<source_location>)
#include <source_location>
#elif __has_include(<experimental/source_location>)
#include <experimental/source_location>
#else
#include "source_location.hpp"
#endif

#if __has_include("modloader/shared/modloader.hpp")
#include "modloader/shared/modloader.hpp"
#include "modinfo_fmt.hpp"
#define QUEST_MODLOADER

#include "queue/concurrentqueue.h"

#endif

namespace Paper {

    static const std::string_view GLOBAL_TAG = "GLOBAL";

#ifndef NOSTD_SOURCE_LOCATION_HPP
    using sl = std::experimental::source_location;
#else
    using sl = nostd::source_location;
#endif
//
//    template <typename... TArgs>
//    struct FmtStringHackTArgs {
//        fmt::format_string<TArgs...> str;
//        sl loc;
//
//        constexpr FmtStringHackTArgs(fmt::format_string<TArgs...> const& str, const sl& loc = sl::current()) : str(str), loc(loc) {}
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
//        consteval inline BasicFmtStrSrcLoc(const S& s, sl sourceL = sl::current()) : fmt::format_string<TArgs...>(s), sourceLocation(sourceL) {}
//    };

    template<typename... TArgs>
    struct BasicFmtStrSrcLoc : public fmt::format_string<TArgs...> {
        sl sourceLocation;

        template <typename S>
        requires (std::is_convertible_v<const S&, fmt::basic_string_view<char>>)
        consteval inline BasicFmtStrSrcLoc(const S& s, sl const& sourceL = sl::current()) : fmt::format_string<TArgs...>(s), sourceLocation(sourceL) {}
    };

    template <typename... Args>
    using FmtStrSrcLoc = BasicFmtStrSrcLoc<fmt::type_identity_t<Args>...>;

    using TimePoint = std::chrono::system_clock::time_point;

    struct ThreadData {
        constexpr ThreadData(ThreadData const&) = delete;
        constexpr ThreadData(ThreadData&&) = default;

        constexpr ThreadData(fmt::string_view const &str, fmt::format_args const &args, std::thread::id const &threadId,
                   std::string_view const &tag, sl const &loc, fmtlog::LogLevel level,
                   TimePoint const &logTime)
                : str(str), args(args), threadId(threadId), tag(tag), loc(loc), level(level), logTime(logTime) {}

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

        fmt::string_view str;
        fmt::format_args args;
        std::thread::id threadId;
        std::string_view tag;
        sl loc;
        fmtlog::LogLevel level;
        TimePoint logTime;
    };

    struct Logger {
        template<fmtlog::LogLevel level>
        static void vfmtLog(fmt::string_view const str, sl const& sourceLoc, std::string_view const tag, fmt::format_args const& args) noexcept {
            while(!logQueue.enqueue(ThreadData(str, args, std::this_thread::get_id(), tag, sourceLoc, level, std::chrono::system_clock::now())));
        }

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr static void fmtLogTag(FmtStrSrcLoc<TArgs...> str, std::string_view const tag, TArgs&&... args) {
            return Logger::vfmtLog<lvl>(str, str.sourceLocation, tag, fmt::make_format_args(str, std::forward<TArgs>(args)...));
        }

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr static void fmtLog(FmtStrSrcLoc<TArgs...> str, TArgs&&... args) {
            return fmtLogTag<lvl, TArgs...>(str, GLOBAL_TAG, std::forward<TArgs>(args)...);
        }

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline void static fmtThrowError(FmtStrSrcLoc<TArgs...> const& str, TArgs&&... args) {
            Logger::fmtLog<fmtlog::LogLevel::ERR, TArgs...>(str, std::forward<TArgs>(args)...);
            throw Exception(fmt::format<TArgs...>(str, std::forward<TArgs>(args)...));
        }

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline void static fmtThrowErrorTag(FmtStrSrcLoc<TArgs...> const& str, fmt::string_view tag, TArgs&&... args) {
            Logger::fmtLog<fmtlog::LogLevel::ERR, TArgs...>(str, tag, std::forward<TArgs>(args)...);

            auto exceptionMsg = fmt::format<TArgs...>(str.str, tag, std::forward<TArgs>(args)...);
            throw Exception(fmt::format("{} {}", tag, exceptionMsg));
        }

        static std::string_view getLogDirectoryPathGlobal();

        static void Init(std::string_view logPath, std::string_view globalLogFileName = "PaperLog.log");

        static void RegisterContextId(std::string_view contextId, std::string_view logPath);

        static inline auto RegisterContextId(std::string_view contextId) {
            return Logger::RegisterContextId(contextId, contextId);
        }

        static void UnregisterContextId(std::string_view contextId);

    private:
        static void LogThread();

        static moodycamel::ConcurrentQueue<ThreadData> logQueue;
    };


    template<std::size_t sz>
    struct LoggerContext {
        char str[sz];

        constexpr LoggerContext(char const (&s)[sz]) {
            std::copy(s, s + sz, str);
        }

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr auto fmtLog(FmtStrSrcLoc<TArgs...> const& str, TArgs&&... args) {
            return Logger::fmtLogTag<lvl, TArgs...>(str, str, std::forward<TArgs>(args)...);
//            Logger::fmtLog<lvl>(fmt::format(FMT_STRING("[{}] {}"), context, fmt::format<TArgs...>(str, std::forward<TArgs>(args)...)));
        }

        template<fmtlog::LogLevel lvl, typename Exception = std::runtime_error, typename... TArgs>
        inline auto fmtThrowError(FmtStrSrcLoc<TArgs...> const& str, TArgs&&... args) {
            return Logger::fmtThrowErrorTag<lvl, Exception, TArgs...>(str, str, std::forward<TArgs>(args)...);
        }
    };

#ifdef QUEST_MODLOADER
//    using ModloaderLoggerContext = BasicLoggerContext<ModInfo const&>;
//
//
//    namespace Logger {
//        inline ModloaderLoggerContext getModloaderContext(ModInfo const& info) {
//            ModloaderLoggerContext context(info);
//
//            Logger::RegisterContextId(fmt::format("{}_{}.log", info.id, info.version));
//
//            return context;
//        }
//    }

#endif
}
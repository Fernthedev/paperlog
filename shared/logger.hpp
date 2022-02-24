#pragma once

#include <fstream>
#include <iostream>
#include <string_view>
#include <fmt/core.h>

#include <fmtlog/fmtlog.h>

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

#endif

namespace Paper {

#ifndef NOSTD_SOURCE_LOCATION_HPP
    using sl = std::experimental::source_location;
#else
    using sl = nostd::source_location;
#endif

    struct FmtStringHack {
        fmt::string_view str;
        sl loc;

        constexpr FmtStringHack(const char* str, const sl& loc = sl::current())
                : str(str), loc(loc) {}

        constexpr FmtStringHack(std::string_view str, const sl& loc = sl::current())
                : str(str), loc(loc) {}
    };

    template <typename... TArgs>
    struct FmtStringHackTArgs {
        fmt::format_string<TArgs...> str;
        sl loc;

        constexpr FmtStringHackTArgs(fmt::format_string<TArgs...> str, const sl& loc = sl::current()) : str(str), loc(loc) {}
    };

    namespace Logger {

        template<fmtlog::LogLevel level>
        static void vfmtLog(FmtStringHack const& stringHack, std::string_view const tag, fmt::format_args const& args) noexcept {
            auto const& location = stringHack.loc;
            auto const& str = stringHack.str;

            // fmtlog hack so we can hijack location and use it as a context id
            do {
                static uint32_t logId = 0;
                if (!fmtlog::checkLogLevel(level))break;
                fmtlogWrapper<>::impl.log(logId, fmtlogT<0>::TSCNS::rdtsc(), tag.data(), level,
                                          "[{}.{}:{}:{}]: {}", location.file_name(), location.function_name(), location.line(),
                                          location.column(), fmt::vformat(str, args));
            }
            while (0);

//            FMTLOG(level, "{}.{} {}:{} {}", location.file_name(), location.function_name(), location.line(), location.column(), fmt::vformat(str, args));
        }

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr static void fmtLogTag(FmtStringHack const& str, std::string_view const tag, TArgs&&... args) {
            // TODO: Compile time check fmt args

            return Logger::vfmtLog<lvl>(str, tag, fmt::make_args_checked<TArgs...>(str.str, std::forward<TArgs>(args)...));
        }

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr static void fmtLog(FmtStringHack const& str, TArgs&&... args) {
            return fmtLogTag<lvl, TArgs...>(str, "", std::forward<TArgs>(args)...);
        }

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline void static fmtThrowError(FmtStringHack const& str, TArgs&&... args) {
            Logger::fmtLog<fmtlog::LogLevel::ERR, TArgs...>(str, std::forward<TArgs>(args)...);
            throw Exception(fmt::format<TArgs...>(str, std::forward<TArgs>(args)...));
        }

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline void static fmtThrowErrorTag(FmtStringHack const& str, fmt::string_view tag, TArgs&&... args) {
            Logger::fmtLog<fmtlog::LogLevel::ERR, TArgs...>(str, tag, std::forward<TArgs>(args)...);

            auto exceptionMsg = fmt::format<TArgs...>(str.str, tag, std::forward<TArgs>(args)...);
            throw Exception(fmt::format("{} {}", tag, exceptionMsg));
        }

        std::string_view getLogDirectoryPathGlobal();

        void Init(std::string_view logPath, std::string_view globalLogFileName = "PaperLog.log");

        void RegisterContextId(std::string_view contextId, std::string_view logPath);

        inline auto RegisterContextId(std::string_view contextId) {
            return Logger::RegisterContextId(contextId, contextId);
        }

        void UnregisterContextId(std::string_view contextId);
    }

    template<typename ContextType>
    struct BasicLoggerContext {
        using ContextT = ContextType;

        ContextT context;

        constexpr BasicLoggerContext(ContextT const& c) : context(c) {}


        // TODO: Fix context not actually applying

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr auto fmtLog(FmtStringHack const& stringHack, TArgs&&... args) {
            auto const& loc = stringHack.loc;
            auto const& str = stringHack.str;

            // TODO: Compile time check fmt args
            auto msg = fmt::vformat(str, fmt::make_format_args(args...));

            return Logger::fmtLogTag<lvl>(FmtStringHack("{}", loc), fmt::to_string(context), msg);
//            Logger::fmtLog<lvl>(fmt::format(FMT_STRING("[{}] {}"), context, fmt::format<TArgs...>(str, std::forward<TArgs>(args)...)));
        }

        template<fmtlog::LogLevel lvl, typename Exception = std::runtime_error, typename... TArgs>
        inline auto fmtThrowError(FmtStringHack const& stringHack, TArgs&&... args) {
            auto const& loc = stringHack.loc;
            auto const& str = stringHack.str;

            // TODO: Compile time check fmt args
            auto msg = fmt::vformat(str, fmt::make_format_args(args...));

            return Logger::fmtThrowErrorTag<lvl, Exception>(FmtStringHack("{}", loc), fmt::to_string(context), msg);
        }
    };

#ifdef QUEST_MODLOADER
    using ModloaderLoggerContext = BasicLoggerContext<ModInfo const&>;


    namespace Logger {
        inline ModloaderLoggerContext getModloaderContext(ModInfo const& info) {
            ModloaderLoggerContext context(info);

            Logger::RegisterContextId(fmt::format("{}_{}.log", info.id, info.version));

            return context;
        }
    }

#endif
}
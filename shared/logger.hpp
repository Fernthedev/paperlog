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

    namespace Logger {
        template<typename Exception = std::runtime_error, typename... TArgs>
        inline void static fmtThrowError(fmt::format_string<TArgs...> str, TArgs&&... args) {
            fmtLog<fmtlog::LogLevel::ERR, TArgs...>(str, std::forward<TArgs>(args)...);
            throw Exception(fmt::format<TArgs...>(str, std::forward<TArgs>(args)...));
        }

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr static void fmtLog(fmt::format_string<TArgs...> str, TArgs&&... args) {
            return vfmtLog<lvl>(str, fmt::make_format_args(args...));
        }

        template<fmtlog::LogLevel level>
        static void vfmtLog(FmtStringHack const& stringHack, fmt::format_args const& args) noexcept {
            auto const& location = stringHack.loc;
            auto const& str = stringHack.str;
            FMTLOG(level, "{}.{} {}:{} {}", location.file_name(), location.function_name(), location.line(), location.column(), fmt::vformat(str, args));
        }

        static std::string_view getLogPathGlobal();

        static void Init(std::string_view logPath, std::string_view globalLogFileName = "PaperLog.log");

    };

    template<typename ContextType>
    struct BasicLoggerContext {
        using ContextT = ContextType;

        ContextT context;
        std::ofstream fileLog;

        constexpr BasicLoggerContext(ContextT const& c) : context(c) {}
        constexpr BasicLoggerContext(ContextT const& c, std::string_view logName)
        : context(c), fileLog(fmt::format("{}/{}", Logger::getLogPathGlobal(), logName)) {}


        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr auto fmtLog(fmt::format_string<TArgs...> str, TArgs&&... args) {


            return Logger::fmtLog<lvl, decltype(context), TArgs...>("[{}] " + str, context, std::forward<TArgs>(args)...);

//            Logger::fmtLog<lvl>(fmt::format(FMT_STRING("[{}] {}"), context, fmt::format<TArgs...>(str, std::forward<TArgs>(args)...)));
        }

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline auto fmtThrowError(fmt::format_string<TArgs...> str, TArgs&&... args) {
            return Logger::fmtThrowError<Exception, decltype(context), TArgs...>("[{}] " + str, context, std::forward<TArgs>(args)...);
        }
    };

#ifdef QUEST_MODLOADER
    using ModloaderLoggerContext = BasicLoggerContext<ModInfo const&>;


    namespace Logger {
        constexpr ModloaderLoggerContext getContext(ModInfo const& info) {
            return {info};
        }
    }

#endif
}
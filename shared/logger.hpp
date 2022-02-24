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

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline void static fmtThrowError(fmt::format_string<TArgs...> str, fmt::string_view tag, TArgs&&... args) {
            fmtLog<fmtlog::LogLevel::ERR, TArgs...>(str, tag, std::forward<TArgs>(args)...);
            throw Exception(fmt::format<std::string_view, TArgs...>("{} " + str, tag, std::forward<TArgs>(args)...));
        }

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr static void fmtLog(fmt::format_string<TArgs...> str, TArgs&&... args) {
            return vfmtLog<lvl>(str, "", fmt::make_format_args(args...));
        }

        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr static void fmtLog(fmt::format_string<TArgs...> str, std::string_view const tag, TArgs&&... args) {
            return vfmtLog<lvl>(str, tag, fmt::make_format_args(args...));
        }

        template<fmtlog::LogLevel level>
        static void vfmtLog(FmtStringHack const& stringHack, std::string_view const tag, fmt::format_args const& args) noexcept {
            auto const& location = stringHack.loc;
            auto const& str = stringHack.str;
            // fmtlog hack so we can hijack location and use it as a mod id

            do {
                static uint32_t logId = 0;
                if (!fmtlog::checkLogLevel(level))break;
                fmtlogWrapper<>::impl.log(logId, fmtlogT<0>::TSCNS::rdtsc(), tag.data(), level,
                                          "{}.{} {}:{} {}", location.file_name(), location.function_name(), location.line(),
                                          location.column(), fmt::vformat(str, args));
            }
            while (0);

//            FMTLOG(level, "{}.{} {}:{} {}", location.file_name(), location.function_name(), location.line(), location.column(), fmt::vformat(str, args));
        }

        static std::string_view getLogDirectoryPathGlobal();

        static void Init(std::string_view logPath, std::string_view globalLogFileName = "PaperLog.log");

        static void RegisterContextId(std::string_view contextId, std::string_view logPath);

        static void UnregisterContextId(std::string_view contextId);

        inline static auto RegisterContextId(std::string_view contextId) {
            return RegisterContextId(contextId, contextId);
        }

    };

    template<typename ContextType>
    struct BasicLoggerContext {
        using ContextT = ContextType;

        ContextT context;

        constexpr BasicLoggerContext(ContextT const& c) : context(c) {}


        template<fmtlog::LogLevel lvl, typename... TArgs>
        constexpr auto fmtLog(fmt::format_string<TArgs...> str, TArgs&&... args) {
            return Logger::fmtLog<lvl, TArgs...>("[{}] " + str, fmt::to_string(context), std::forward<TArgs>(args)...);
//            Logger::fmtLog<lvl>(fmt::format(FMT_STRING("[{}] {}"), context, fmt::format<TArgs...>(str, std::forward<TArgs>(args)...)));
        }

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline auto fmtThrowError(fmt::format_string<TArgs...> str, TArgs&&... args) {
            return Logger::fmtThrowError<Exception, TArgs...>("[{}] " + str, fmt::to_string(context), std::forward<TArgs>(args)...);
        }
    };

#ifdef QUEST_MODLOADER
    using ModloaderLoggerContext = BasicLoggerContext<ModInfo const&>;


    namespace Logger {
        inline ModloaderLoggerContext getModloaderContext(ModInfo const& info) {
            ModloaderLoggerContext context(info);

            Logger::RegisterContextId(fmt::to_string(info));

            return context;
        }
    }

#endif
}
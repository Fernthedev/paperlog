#pragma once

#include <fmt/core.h>
#include "internal_logger.hpp"

#if __has_include("modloader/shared/modloader.hpp")
#include "modloader/shared/modloader.hpp"
#include "modinfo_fmt.hpp"
#define QUEST_MODLOADER
#endif

//#include <fmtlog/fmtlog.h>

namespace Paper {

    enum class LogLevel : uint8_t
    {
        DBG = 0,
        INF,
        WRN,
        ERR,
        OFF
    };

    static constexpr const std::string_view GLOBAL_TAG = "GLOBAL";
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


    // TODO: Inherit when NDK fixes bug
    // https://github.com/android/ndk/issues/1677
    template<typename Char, typename... TArgs>
    struct BasicFmtStrSrcLoc {
        using ParentType = fmt::basic_format_string<Char, TArgs...>;
        ParentType parentType;

        explicit(false) inline operator ParentType&() {
            return parentType;
        }

        explicit(false) inline operator ParentType const&() const {
            return parentType;
        }

        explicit(false) inline operator fmt::string_view () {
            return parentType;
        }

        explicit(false) inline operator fmt::string_view const () const {
            return parentType;
        }

        sl sourceLocation;

        template <typename S>
        requires (std::is_convertible_v<const S&, fmt::basic_string_view<char>>)
        consteval inline BasicFmtStrSrcLoc(const S& s, sl const& sourceL = sl::current()) : parentType(s), sourceLocation(sourceL) {}

        BasicFmtStrSrcLoc(fmt::basic_runtime<char> r, sl const& sourceL = sl::current()) : parentType(r), sourceLocation(sourceL) {}
    };

//    template <typename... Args>
//    using FmtStrSrcLoc = BasicFmtStrSrcLoc<char, fmt::type_identity_t<Args>...>;
    template <typename... Args>
    using FmtStrSrcLoc = BasicFmtStrSrcLoc<char, fmt::type_identity_t<Args>...>;


    namespace Logger {
        template<LogLevel level>
        constexpr inline void vfmtLog(fmt::string_view const str, sl const& sourceLoc, std::string_view const tag, fmt::format_args const& args) noexcept {
            while(!Internal::logQueue.enqueue(ThreadData(str, args, std::this_thread::get_id(), tag, sourceLoc, level, std::chrono::system_clock::now())));
        }

        template<LogLevel lvl, typename... TArgs>
        constexpr auto fmtLogTag(FmtStrSrcLoc<TArgs...> str, std::string_view const tag, TArgs&&... args) {
            return Logger::vfmtLog<lvl>(str, str.sourceLocation, tag, fmt::make_format_args(str.parentType, std::forward<TArgs>(args)...));
        }

        template<LogLevel lvl, typename... TArgs>
        constexpr auto fmtLog(FmtStrSrcLoc<TArgs...> str, TArgs&&... args) {
            return fmtLogTag<lvl, TArgs...>(str, GLOBAL_TAG, std::forward<TArgs>(args)...);
        }

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline void fmtThrowError(FmtStrSrcLoc<TArgs...> const& str, TArgs&&... args) {
            Logger::fmtLog<LogLevel::ERR, TArgs...>(str, std::forward<TArgs>(args)...);
            throw Exception(fmt::format<TArgs...>(str, std::forward<TArgs>(args)...));
        }

        template<typename Exception = std::runtime_error, typename... TArgs>
        inline void fmtThrowErrorTag(FmtStrSrcLoc<TArgs...> const& str, fmt::string_view tag, TArgs&&... args) {
            Logger::fmtLog<LogLevel::ERR, TArgs...>(str, tag, std::forward<TArgs>(args)...);

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
    };


    template<std::size_t sz>
    struct LoggerContext {
        char str[sz];

        constexpr LoggerContext(char const (&s)[sz]) {
            std::copy(s, s + sz, str);
        }

        template<LogLevel lvl, typename... TArgs>
        constexpr auto fmtLog(FmtStrSrcLoc<TArgs...> const& str, TArgs&&... args) {
            return Logger::fmtLogTag<lvl, TArgs...>(str, str, std::forward<TArgs>(args)...);
//            Logger::fmtLog<lvl>(fmt::format(FMT_STRING("[{}] {}"), context, fmt::format<TArgs...>(str, std::forward<TArgs>(args)...)));
        }

        template<LogLevel lvl, typename Exception = std::runtime_error, typename... TArgs>
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
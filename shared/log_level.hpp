#pragma once

#include <fmt/format.h>
#include <string_view>

namespace Paper {
    enum class LogLevel : uint8_t
    {
        DBG = 3,
        INF = 4,
        WRN = 5,
        ERR = 6,
        CRIT = 7,
        OFF = 0
    };
}

template <>
struct fmt::formatter<Paper::LogLevel> : formatter<string_view>
{

    // Formats the point p using the parsed format specification (presentation)
    // stored in this formatter.
    template <typename FormatContext>
    auto format(Paper::LogLevel level, FormatContext &ctx) -> decltype(ctx.out())
    {
        using namespace Paper;

        std::string_view levelStr = "UNKNOWN";
        switch (level)
        {
        case LogLevel::DBG:
            levelStr = "DEBUG";
            break;

        case LogLevel::INF:
            levelStr = "INFO";
            break;

        case LogLevel::WRN:
            levelStr = "WARN";
            break;

        case LogLevel::ERR:
            levelStr = "ERROR";
            break;

        case LogLevel::CRIT:
            levelStr = "CRITICAL";
            break;
        default:
            break;
        }

        return formatter<string_view>::format(levelStr, ctx);
    }
};
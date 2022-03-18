#pragma once

#include <fmt/format.h>

namespace Paper
{
    enum class LogLevel : uint8_t
    {
        DBG = 2,
        INF = 3,
        WRN = 4,
        ERR = 5,
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
        default:
            break;
        }

        return formatter<string_view>::format(levelStr, ctx);
    }
};
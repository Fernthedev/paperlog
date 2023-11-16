#pragma once

#include <fmt/format.h>
#include <string_view>

namespace Paper {
enum class LogLevel : uint8_t { DBG = 3, INF = 4, WRN = 5, ERR = 6, CRIT = 7, OFF = 0 };

constexpr auto format_as(LogLevel level) {
  switch (level) {
  case LogLevel::DBG:
    return "DEBUG";
    break;

  case LogLevel::INF:
    return "INFO";
    break;

  case LogLevel::WRN:
    return "WARN";
    break;

  case LogLevel::ERR:
    return "ERROR";
    break;

  case LogLevel::CRIT:
    return "CRITICAL";
    break;
  default:
    return "UNKNOWN";
  }
}
} // namespace Paper
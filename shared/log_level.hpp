#pragma once

#include <fmt/base.h>
#include <string_view>

#include "bindings.h"

namespace Paper {
enum class LogLevel : uint8_t {
  DBG = Paper::ffi::paper2_LogLevel::Debug,
  INF = Paper::ffi::paper2_LogLevel::Info,
  WRN = Paper::ffi::paper2_LogLevel::Warn,
  ERR = Paper::ffi::paper2_LogLevel::Error,
  CRIT = Paper::ffi::paper2_LogLevel::Crit,
  OFF = Paper::ffi::paper2_LogLevel::Off
};

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
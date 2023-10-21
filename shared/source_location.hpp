// yoinked from https://github.com/paweldac/source_location/blob/master/include/source_location/source_location.hpp

#ifndef NOSTD_SOURCE_LOCATION_HPP
#define NOSTD_SOURCE_LOCATION_HPP

#pragma once

#include <cstdint>
#include <string_view>

namespace nostd {
struct source_location {
public:
#if not defined(__apple_build_version__) and defined(__clang__) and (__clang_major__ >= 9)
  static constexpr source_location current(char const* fileName = __builtin_FILE(),
                                           char const* functionName = __builtin_FUNCTION(),
                                           const uint_least32_t lineNumber = __builtin_LINE(),
                                           const uint_least32_t columnOffset = __builtin_COLUMN()) noexcept
#elif defined(__GNUC__) and (__GNUC__ > 4 or (__GNUC__ == 4 and __GNUC_MINOR__ >= 8))
  static constexpr source_location
  current(char const* fileName = __builtin_FILE(), char const* functionName = __builtin_FUNCTION(),
          const uint_least32_t lineNumber = __builtin_LINE(), const uint_least32_t columnOffset = 0) noexcept
#else
  static constexpr source_location
  current(char const* fileName = "unsupported", char const* functionName = "unsupported",
          const uint_least32_t lineNumber = 0, const uint_least32_t columnOffset = 0) noexcept
#endif
  {
    return source_location(fileName, functionName, lineNumber, columnOffset);
  }

  source_location(source_location const&) = default;
  source_location(source_location&&) = default;

  source_location& operator=(source_location const&) = default;
  source_location& operator=(source_location&&) = default;

  [[nodiscard]] constexpr std::string_view file_name() const noexcept {
    return fileName;
  }

  [[nodiscard]] constexpr std::string_view function_name() const noexcept {
    return functionName;
  }

  [[nodiscard]] constexpr uint_least32_t line() const noexcept {
    return lineNumber;
  }

  [[nodiscard]] constexpr std::uint_least32_t column() const noexcept {
    return columnOffset;
  }

private:
  constexpr source_location(char const* fileName, char const* functionName, const uint_least32_t lineNumber,
                            const uint_least32_t columnOffset) noexcept
      : fileName(fileName), functionName(functionName), lineNumber(lineNumber), columnOffset(columnOffset) {}

  std::string_view fileName;
  std::string_view functionName;
  std::uint_least32_t lineNumber;
  std::uint_least32_t columnOffset;
};
} // namespace nostd

#endif
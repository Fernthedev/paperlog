#pragma once

#ifndef NO_MODLOADER_FORMAT

#include <fmt/base.h>

#include "modloader/shared/modloader.hpp"

template <> struct fmt::formatter<ModInfo> : formatter<string_view> {

  // Formats the point p using the parsed format specification (presentation)
  // stored in this formatter.
  template <typename FormatContext> auto format(ModInfo const& p, FormatContext& ctx) -> decltype(ctx.out()) {
    // ctx.out() is an output iterator to write to.

    return format_to(ctx.out(), "{}, {}", p.id, p.version);
  }
};

#endif
#pragma once

#include <fmt/format.h>

#include "modloader/shared/modloader.hpp"

template <> struct fmt::formatter<ModInfo>: formatter<std::string_view> {

    // Formats the point p using the parsed format specification (presentation)
    // stored in this formatter.
    template <typename FormatContext>
    auto format(ModInfo const& p, FormatContext& ctx) -> decltype(ctx.out()) {
        // ctx.out() is an output iterator to write to.

        return formatter<string_view>::format("{}:{}", p.id, p.version, ctx);
    }
};
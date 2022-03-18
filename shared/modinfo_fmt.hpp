#pragma once

#include <fmt/format.h>

#if __has_include("modloader/shared/modloader.hpp") && __has_include(<jni.h>)
//#include "modloader/shared/modloader.hpp"
#define HAS_QUEST_MODLOADER
#else
struct ModInfo {
    std::string id;
    std::string version;
};
#endif

template <> struct fmt::formatter<ModInfo> : formatter<string_view>{

    // Formats the point p using the parsed format specification (presentation)
    // stored in this formatter.
    template <typename FormatContext>
    auto format(ModInfo const& p, FormatContext& ctx) -> decltype(ctx.out()) {
        // ctx.out() is an output iterator to write to.

        return format_to(ctx.out(), "{}, {}",  p.id, p.version);
    }
};
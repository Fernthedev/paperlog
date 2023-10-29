#pragma once
#ifndef NO_SL2_FORMAT

#include <variant>
#include <fmt/format.h>

#include "scotland2/shared/modloader.h"
#include "scotland2/shared/loader.hpp"

template <> struct fmt::formatter<modloader::LoadedMod> : formatter<string_view> {

  // Formats the point p using the parsed format specification (presentation)
  // stored in this formatter.
  template <typename FormatContext>
  auto format(modloader::LoadedMod const& p, FormatContext& ctx) -> decltype(ctx.out()) {
    // ctx.out() is an output iterator to write to.

    return format_to(ctx.out(), "{}, {} handle {}", p.modInfo, p.phase, p.handle);
  }
};
template <> struct fmt::formatter<modloader::FailedMod> : formatter<string_view> {

  // Formats the point p using the parsed format specification (presentation)
  // stored in this formatter.
  template <typename FormatContext>
  auto format(modloader::FailedMod const& p, FormatContext& ctx) -> decltype(ctx.out()) {
    // ctx.out() is an output iterator to write to.

    return format_to(ctx.out(), "{} error {}", p.object.path.filename(), p.failure);
  }
};
// template <> struct fmt::formatter<modloader::DependencyResult> : formatter<string_view> {

//   // Formats the point p using the parsed format specification (presentation)
//   // stored in this formatter.
//   template <typename FormatContext>
//   auto format(modloader::DependencyResult const& p, FormatContext& ctx) -> decltype(ctx.out()) {
//     // ctx.out() is an output iterator to write to.

//     auto loadedMod = std::get_if<modloader::MissingDependency>(p);
//     if (loadedMod) {
//     }

//     return format_to(ctx.out(), "{} error {}", p.object.path.filename(), p.failure);
//   }
// };

#endif
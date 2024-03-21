#pragma once

#include <string>
#include <fstream>

using ContextID = std::string;
using LogFile = std::ofstream;

constexpr auto GLOBAL_FILE_NAME = "PaperLog.log";

struct StringHash {
  using is_transparent = void; // enables heterogenous lookup
  std::size_t operator()(std::string_view sv) const {
    std::hash<std::string_view> hasher;
    return hasher(sv);
  }
};
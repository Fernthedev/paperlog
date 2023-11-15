#include <android/log.h>

#include "_config.h"
#include "internal.hpp"
#include "logger.hpp"

#ifndef PAPER_QUEST_SCOTLAND2
#error SCOTLAND2 NOT FOUND! RUN `qpm restore` in `bootstrapper/scotland2`!
#endif

#include "scotland2/shared/modloader.h"

namespace {
void findAndReplaceAll(std::string& data, std::string_view toSearch, std::string_view replaceStr) noexcept {
  // Get the first occurrence
  size_t pos = data.find(toSearch);
  // Repeat till end is reached
  while (pos != std::string::npos) {
    // Replace this occurrence of Sub String
    data.replace(pos, toSearch.size(), replaceStr);
    // Get the next occurrence from the current position
    pos = data.find(toSearch, pos + replaceStr.size());
  }
}
} // namespace

void __attribute__((constructor(1000))) dlopen_initialize() {
  WriteStdOut(ANDROID_LOG_INFO, "PAPERLOG", "DLOpen initializing");

  std::string path = "/sdcard/Android/data/{}/files/logs/paper";
  findAndReplaceAll(path, "{}", modloader::get_application_id());

  try {
    Paper::Logger::Init(path, Paper::LoggerConfig());
  } catch (std::exception const& e) {
    std::string error = "Error occurred in logging thread!" + std::string(e.what());
    WriteStdOut(ANDROID_LOG_ERROR, "PAPERLOG", error);
    throw e;
  } catch (...) {
    std::string error = "Error occurred in logging thread!";
    WriteStdOut(ANDROID_LOG_ERROR, "PAPERLOG", error);
    throw;
  }
}
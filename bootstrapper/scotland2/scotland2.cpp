#include <android/log.h>

#include "_config.h"
#include "internal.hpp"
#include "logger.hpp"

#ifndef PAPER_QUEST_SCOTLAND2
#error SCOTLAND2 NOT FOUND! RUN `qpm restore` in `bootstrapper/scotland2`!
#endif

void __attribute__((constructor(1000))) dlopen_initialize() {
  WriteStdOut(ANDROID_LOG_INFO, "PAPERLOG", "DLOpen initializing");


  std::string path = fmt::format("/sdcard/Android/data/{}/files/logs/paper", modloader::get_application_id());

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
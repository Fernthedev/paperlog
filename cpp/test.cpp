
#include <fmt/base.h>
#include "logger.hpp"
#include <chrono>
#include <thread>
#include <iostream>

int main() {
  Paper::Logger::Init("./logs/globalLogFile.txt");

  std::this_thread::sleep_for(std::chrono::milliseconds(500));
  if (!Paper::Logger::IsInited()) {
    std::cerr << "Logger not inited" << std::endl;
    return 1;
  }

  Paper::Logger::info("Test {}", 5);

  bool logged;
  Paper::Logger::AddLogSink([&logged](Paper::LogData data) {
    std::cout << "LogSink: " << data.message << std::endl;
    logged = true;
    return 0;
  });

  Paper::Logger::warn("Test2 {}", 10);
  std::this_thread::sleep_for(std::chrono::milliseconds(500));

  if (!logged) {
    std::cerr << "LogSink not called" << std::endl;
    return 1;
  }

  Paper::Logger::WithContextRuntime("TestContext").error("Test3 {}", 15);

  std::cout << "Success" << std::endl;

  return 0;
}
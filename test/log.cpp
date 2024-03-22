#include <string_view>
#include <source_location>
#include <thread>
#include <chrono>

#include "log_level.hpp"
#include "string_convert.hpp"

#include "logger.hpp"
#include "Profiler.hpp"

#include <gtest/gtest.h>

void testLogSpamLoop(int spamCount) {
  for (int i = 0; i < spamCount; i++) {
    Paper::Logger::fmtLog<Paper::LogLevel::DBG>("Multithread spam! {}", i);
  }
}

void testMultithreadedSpam(int threadCount, int spamCount) {
  Paper::Profiler<std::chrono::nanoseconds> profiler;
  profiler.suffix = "ns";

  std::vector<std::thread> threads;
  threads.reserve(threadCount);
  for (int i = 0; i < threadCount; i++) {
    threads.emplace_back(&testLogSpamLoop, spamCount);
  }

  for (auto& t : threads) {
    if (!t.joinable()) continue;
    t.join();
  }
  profiler.mark("Threads finished logging");
  Paper::Logger::WaitForFlush();
  profiler.mark("All logs flushed");
  profiler.printMarks();
}

void WaitForCompleteFlush() {
  using namespace std::chrono_literals;

  std::this_thread::sleep_for(2ms);
  Paper::Logger::WaitForFlush();
  while (Paper::Internal::logQueue.size_approx() > 0) {
    std::this_thread::sleep_for(2ms);
  }
}

TEST(LogTest, LogInit) {
  Paper::Logger::Init(".");
  std::this_thread::sleep_for(std::chrono::milliseconds(100));
  Paper::Logger::WaitForFlush();
  EXPECT_EQ(Paper::Logger::IsInited(), true);
}

TEST(LogTest, LogOutput) {
  Paper::Logger::Init(".");
  std::cout.clear();
  testing::internal::CaptureStdout();
  Paper::Logger::fmtLog<Paper::LogLevel::INF>("hi! {}", 5);
  WaitForCompleteFlush();
  std::string output = testing::internal::GetCapturedStdout();
  EXPECT_EQ(output, "Level (INFO) [GLOBAL] hi! 5\n");
}

TEST(LogTest, SingleThreadLogSpam) {
  std::cout.clear();
  testing::internal::CaptureStdout();

  Paper::Profiler<std::chrono::nanoseconds> profiler;
  profiler.suffix = "ns";
  profiler.startTimer();

  Paper::Logger::fmtLog<Paper::LogLevel::DBG>("Spam logging now!");
  for (int i = 0; i < 100000; i++) {
    Paper::Logger::fmtLog<Paper::LogLevel::DBG>("log i {}", i);
  }
  WaitForCompleteFlush();
  profiler.endTimer();

  std::string output = testing::internal::GetCapturedStdout();
  std::cout << "Single thread took "
            << std::chrono::duration_cast<std::chrono::milliseconds>(profiler.elapsedTime()).count() << "ms"
            << std::endl;
}
TEST(LogTest, MultiThreadLogSpam) {
  std::cout.clear();
  testing::internal::CaptureStdout();

  Paper::Profiler<std::chrono::nanoseconds> profiler;
  profiler.suffix = "ns";
  profiler.startTimer();

  Paper::Logger::fmtLog<Paper::LogLevel::DBG>("Spam logging now!");
  testMultithreadedSpam(4, 100000);
  WaitForCompleteFlush();
  profiler.endTimer();

  std::string output = testing::internal::GetCapturedStdout();
  std::cout << "Multithread thread took "
            << std::chrono::duration_cast<std::chrono::milliseconds>(profiler.elapsedTime()).count() << "ms"
            << std::endl;
}

TEST(LogTest, LogContextOutput) {
  std::cout.clear();
  auto context = Paper::Logger::WithContext<"Context">();

  testing::internal::CaptureStdout();
  context.fmtLog<Paper::LogLevel::INF>("context hi! {}", 6);
  WaitForCompleteFlush();

  std::string output = testing::internal::GetCapturedStdout();
  EXPECT_EQ(output, "Level (INFO) [" + std::string(context.tag) + "] context hi! 6\n");
}
TEST(LogTest, LogContextTagOutput) {
  std::cout.clear();
  testing::internal::CaptureStdout();
  std::string context = "Context";

  Paper::Logger::fmtLogTag<Paper::LogLevel::INF>("hi this is a context log! {}", context, 5);
  WaitForCompleteFlush();
  std::string output = testing::internal::GetCapturedStdout();
  EXPECT_EQ(output, "Level (INFO) [" + context + "] hi this is a context log! 5\n");
}

// TODO: Fix
// TEST(LogTest, LogOutputLinebreaks) {
//     std::cout.clear();
//     testing::internal::CaptureStdout();
//     Paper::Logger::fmtLog<Paper::LogLevel::INF>("5 \n\n\n\n\nlines");
//     WaitForCompleteFlush();

//     std::string output = testing::internal::GetCapturedStdout();
//     std::cout << output << std::endl;
//     EXPECT_EQ(output, "5 \n\n\n\n\nlines\n");
// }
TEST(LogTest, UTF8) {
  std::cout.clear();
  testing::internal::CaptureStdout();
  Paper::Logger::fmtLog<Paper::LogLevel::INF>("£ ह € 한");
  WaitForCompleteFlush();

  std::string output = testing::internal::GetCapturedStdout();
  EXPECT_EQ(output, "Level (INFO) [GLOBAL] £ ह € 한\n");
}
TEST(LogTest, UTF16ToUTF8) {
  std::cout.clear();
  testing::internal::CaptureStdout();
  Paper::Logger::fmtLog<Paper::LogLevel::INF>("Testing UTF-16 conversion chars {}",
                                              Paper::StringConvert::from_utf16(u"한🌮🦀"));
  WaitForCompleteFlush();

  std::string output = testing::internal::GetCapturedStdout();
  EXPECT_EQ(output, "Level (INFO) [GLOBAL] Testing UTF-16 conversion chars 한🌮🦀\n");
  EXPECT_EQ(output, "Level (INFO) [GLOBAL] Testing UTF-16 conversion chars " +
                        Paper::StringConvert::from_utf16(u"한🌮🦀") + "\n");
  //  EXPECT_EQ(Paper::StringConvert::from_utf8(output),
  //            u"Testing UTF-16 conversion chars 한🌮🦀\n");
}

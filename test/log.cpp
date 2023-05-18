#include <string_view>
#include <source_location>
#include <thread>
#include <chrono>

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

TEST(LogTest, LogOutput) {
    Paper::Logger::Init(".");
    std::cout.clear();
    testing::internal::CaptureStdout();
    Paper::Logger::fmtLog<Paper::LogLevel::INF>("hi! {}", 5);
    WaitForCompleteFlush();
    std::string output = testing::internal::GetCapturedStdout();
    EXPECT_EQ(output, "hi! 5\n");
}
TEST(LogTest, LogOutputLinebreaks) {
    std::cout.clear();
    testing::internal::CaptureStdout();
    Paper::Logger::fmtLog<Paper::LogLevel::INF>("5 \n\n\n\n\nlines");
    WaitForCompleteFlush();

    std::string output = testing::internal::GetCapturedStdout();
    std::cout << output << std::endl;
    EXPECT_EQ(output, "5 \n\n\n\n\nlines\n");
}
TEST(LogTest, UTF8) {
    std::cout.clear();
    testing::internal::CaptureStdout();
    Paper::Logger::fmtLog<Paper::LogLevel::INF>("£ ह € 한");
    WaitForCompleteFlush();

    std::string output = testing::internal::GetCapturedStdout();
    EXPECT_EQ(output, "£ ह € 한\n");
}
TEST(LogTest, UTF16ToUTF8) {
    std::cout.clear();
     testing::internal::CaptureStdout();
     Paper::Logger::fmtLog<Paper::LogLevel::INF>(
         "Testing UTF-16 conversion chars {}",
         Paper::StringConvert::from_utf16(u"한🌮🦀"));
     WaitForCompleteFlush();

     std::string output = testing::internal::GetCapturedStdout();
     EXPECT_EQ(output, "Testing UTF-16 conversion chars 한🌮🦀\n");
     EXPECT_EQ(output, "Testing UTF-16 conversion chars " +
                           Paper::StringConvert::from_utf16(u"한🌮🦀") + "\n");
    //  EXPECT_EQ(Paper::StringConvert::from_utf8(output),
    //            u"Testing UTF-16 conversion chars 한🌮🦀\n");
}

// TODO: Spam log
// TODO: Multithread log

// // Called later on in the game loading - a good time to install function hooks
// extern "C" void load() {

//     Paper::Profiler<std::chrono::nanoseconds> profiler;
//     profiler.suffix = "ns";
//     profiler.startTimer();
//     // Test code here assumes we are on quest, as such, the path is already known.
//     // Paper::Logger::Init("/sdcard/Android/data/com.beatgames.beatsaber/files/logs");
//     //        // creates a file logger

//     profiler.mark("Init");

//     Paper::Logger::fmtLog<Paper::LogLevel::INF>("hi! {}", 5);
//     profiler.mark("Sent 1 log");
//     Paper::Logger::fmtLog<Paper::LogLevel::INF>("Paper loaded! {}", "modInfo");
//     Paper::Logger::fmtLog<Paper::LogLevel::INF>("5 \n\n\n\n\nlines", "modInfo");
//     Paper::Logger::fmtLog<Paper::LogLevel::INF>("Testing weird UTF-8 chars £\n"
//                                                 "ह\n"
//                                                 "€\n"
//                                                 "한\n");

//     Paper::Logger::fmtLog<Paper::LogLevel::INF>("Testing UTF-16 conversion chars {}",
//                                                 Paper::StringConvert::from_utf16(u"한🌮🦀"));

//     Paper::Logger::WaitForFlush();
//     profiler.mark("Flushed");
//     Paper::Logger::Backtrace(5);
//     profiler.mark("Backtrace");

//     auto fastContext = Paper::Logger::WithContext<"PaperFast">();
//     fastContext.fmtLog<Paper::LogLevel::INF>("Paper fast!");
//     profiler.printMarks();




//     profiler = {};
//     profiler.suffix = "ns";
//     profiler.startTimer();
//     fastContext.fmtLog<Paper::LogLevel::DBG>("Spam logging now!");
//     for (int i = 0; i < 100000; i++) {
//         fastContext.fmtLog<Paper::LogLevel::DBG>("log i {}", i);
//     }
//     profiler.mark("Logged 100000 times");
//     Paper::Logger::WaitForFlush();
//     profiler.mark("Flushed 100000 logs");
//     profiler.printMarks();

//     Paper::Logger::fmtLog<Paper::LogLevel::INF>(fmt::runtime(std::string(10, '\n')));
//     testMultithreadedSpam(4, 400);
// }
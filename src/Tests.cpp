#include "Tests.hpp"

#include "string_convert.hpp"
#include "logger.hpp"
#include "Profiler.hpp"

#include "modinfo_fmt.hpp"

using namespace Paper;

#ifdef HAS_QUEST_MODLOADER
static ModInfo modInfo{"Paper!", "1.0.0"};
#endif

void testLogSpamLoop(int spamCount) {
    for (int i = 0; i < spamCount; i++) {
        Logger::fmtLog<LogLevel::DBG>("Multithread spam! {}", i);
    }
}

void testMultithreadedSpam(int threadCount, int spamCount) {
    Profiler<std::chrono::nanoseconds> profiler;
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
    Logger::WaitForFlush();
    profiler.mark("All logs flushed");
    profiler.printMarks();
}

void Test::runTests() {

    Profiler<std::chrono::nanoseconds> profiler;
    profiler.suffix = "ns";
    profiler.startTimer();
    Logger::Init("/sdcard/Android/data/com.beatgames.beatsaber/files/logs");
    //        // creates a file logger

    profiler.mark("Init");

    Logger::fmtLog<LogLevel::INF>("hi! {}", 5);
    profiler.mark("Sent 1 log");
#ifdef HAS_QUEST_MODLOADER
    Logger::fmtLog<LogLevel::INF>("Paper loaded! {}", modInfo);
    Logger::fmtLog<LogLevel::INF>("5 \n\n\n\n\nlines", modInfo);
#endif

    Logger::fmtLog<LogLevel::INF>("Testing weird UTF-8 chars £\n"
                                                "ह\n"
                                                "€\n"
                                                "한\n");

    Logger::fmtLog<LogLevel::INF>("Testing UTF-16 conversion chars {}",
                                                StringConvert::from_utf16(u"한🌮🦀"));

    Logger::WaitForFlush();
    profiler.mark("Flushed");
    Logger::Backtrace(5);
    profiler.mark("Backtrace");

    auto fastContext = Logger::WithContext<"PaperFast">();
    fastContext.fmtLog<LogLevel::INF>("Paper fast!");
    profiler.printMarks();




    profiler = {};
    profiler.suffix = "ns";
    profiler.startTimer();
    fastContext.fmtLog<LogLevel::DBG>("Spam logging now!");
    for (int i = 0; i < 100000; i++) {
        fastContext.fmtLog<LogLevel::DBG>("log i {}", i);
    }
    profiler.mark("Logged 100000 times");
    Logger::WaitForFlush();
    profiler.mark("Flushed 100000 logs");
    profiler.printMarks();

    Logger::fmtLog<LogLevel::INF>(fmt::runtime(std::string(10, '\n')));
    testMultithreadedSpam(4, 100000);
}
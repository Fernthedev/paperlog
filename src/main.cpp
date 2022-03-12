#include "main.hpp"

#include "modinfo_fmt.hpp"
#include "logger.hpp"
#include "Profiler.hpp"

static ModInfo modInfo; // Stores the ID and version of our mod, and is sent to the modloader upon startup


// Called at the early stages of game loading
extern "C" void setup(ModInfo& info) {
    info.id = MOD_ID;
    info.version = VERSION;
    modInfo = info;

}

// Called later on in the game loading - a good time to install function hooks
extern "C" void load() {

    Paper::Profiler<std::chrono::nanoseconds> profiler;
    profiler.suffix = "ns";
    profiler.startTimer();
    Paper::Logger::Init("/sdcard/Android/data/com.beatgames.beatsaber/files/logs");
    //        // creates a file logger

    profiler.mark("Init");

    Paper::Logger::fmtLog<Paper::LogLevel::INF>("hi! {}", 5);
    profiler.mark("Sent 1 log");
    Paper::Logger::fmtLog<Paper::LogLevel::INF>("Paper loaded! {}", modInfo);
    Paper::Logger::fmtLog<Paper::LogLevel::INF>("5 \n\n\n\n\nlines", modInfo);
    Paper::Logger::fmtLog<Paper::LogLevel::INF>("Testing weird UTF-8 chars £\n"
                                                "ह\n"
                                                "€\n"
                                                "한\n");

    Paper::Logger::WaitForFlush();
    profiler.mark("Flushed");
    Paper::Logger::Backtrace(5);
    profiler.mark("Backtrace");

    auto fastContext = Paper::Logger::WithContext<"PaperFast">();
    fastContext.fmtLog<Paper::LogLevel::INF>("Paper fast!");
    profiler.printMarks();




    profiler = {};
    profiler.suffix = "ns";
    profiler.startTimer();
    fastContext.fmtLog<Paper::LogLevel::DBG>("Spam logging now!");
    for (int i = 0; i < 100000; i++) {
        fastContext.fmtLog<Paper::LogLevel::DBG>("log i {}", i);
    }
    profiler.mark("Logged 100000 times");
    Paper::Logger::WaitForFlush();
    profiler.mark("Flushed 100000 logs");
    profiler.printMarks();
}
#include "logger.hpp"

#include <fmt/ostream.h>

#include <optional>
#include <fstream>
#include <iostream>
#include <android/log.h>
#include <thread>


struct StringHash {
    using is_transparent = void; // enables heterogenous lookup
    std::size_t operator()(std::string_view sv) const {
        std::hash<std::string_view> hasher;
        return hasher(sv);
    }
};

static std::string globalLogPath;
using ContextID = std::string;
using LogPath = std::ofstream;

static std::unordered_map<ContextID, LogPath, StringHash, std::equal_to<>> registeredContexts;

// callback signature user can register
// ns: nanosecond timestamp
// level: logLevel
// location: full file path with line num, e.g: /home/raomeng/fmtlog/fmtlog.h:45
// basePos: file base index in the location
// threadName: thread id or the name user set with setThreadName
// msg: full log msg with header
// bodyPos: log body index in the msg
// logFilePos: log file position of this msg
void OnLog(int64_t ns, fmtlog::LogLevel level, fmt::string_view location, size_t basePos,
           fmt::string_view threadName, fmt::string_view msg, size_t bodyPos, size_t logFilePos) {
    __android_log_print(level, fmt::format("{}", location).c_str(), "%s", msg.data());

    auto it = registeredContexts.find(location.data());
    if (it != registeredContexts.end()) {
        auto& f = it->second;
        fmt::print(f, FMT_STRING("{}"), msg);
    }
}

//void LogThread() {
//    fmtlog::startPollingThread(interval)
//    while(true) {
//        std::this_thread::yield();
//    }
//}

void Paper::Logger::Init(std::string_view logPath, std::string_view globalLogFileName) {
    globalLogPath = logPath;

    fmtlog::setHeaderPattern("{md} [{HMSf}] {l}[{t:<6}] {s:<16}:");
    fmtlog::setLogFile(fmt::format("{}/{}", logPath, globalLogFileName).c_str(), false);
    fmtlog::setLogCB(OnLog, fmtlog::LogLevel::DBG);

    // I don't think we need a custom polling thread
    fmtlog::startPollingThread();
}

std::string_view Paper::Logger::getLogDirectoryPathGlobal() {
    return globalLogPath;
}

// TODO: Lock?
void Paper::Logger::RegisterContextId(std::string_view contextId, std::string_view logPath) {
    registeredContexts.try_emplace(contextId.data(), fmt::format("{}/{}", getLogDirectoryPathGlobal(), logPath));
}

void Paper::Logger::UnregisterContextId(std::string_view contextId) {
    registeredContexts.erase(contextId.data());
}

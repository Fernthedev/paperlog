#include "logger.hpp"
#include "main.hpp"

#include <fmt/ostream.h>
#include <fmt/chrono.h>

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

moodycamel::ConcurrentQueue<Paper::ThreadData> Paper::Internal::logQueue;

static std::string globalLogPath;
using ContextID = std::string;
using LogPath = std::ofstream;

static std::unordered_map<ContextID, LogPath, StringHash, std::equal_to<>> registeredContexts;
static LogPath globalFile;
//
//// callback signature user can register
//// ns: nanosecond timestamp
//// level: logLevel
//// location: full file path with line num, e.g: /home/raomeng/fmtlog/fmtlog.h:45
//// basePos: file base index in the location
//// threadName: thread id or the name user set with setThreadName
//// msg: full log msg with header
//// bodyPos: log body index in the msg
//// logFilePos: log file position of this msg
//void OnLog(int64_t ns, fmtlog::LogLevel level, fmt::string_view location, size_t basePos,
//           fmt::string_view threadName, fmt::string_view msg, size_t bodyPos, size_t logFilePos) {
//    __android_log_print(level, fmt::format("{}", location).c_str(), "%s", msg.data());
//
//    // TODO: This is slow and bad, figure out something better
//
//}

[[noreturn]]
void Paper::Internal::LogThread() {
    moodycamel::ConsumerToken token(Paper::Internal::logQueue);

    Paper::ThreadData threadData{"",fmt::make_format_args(), std::this_thread::get_id(),"", Paper::sl::current(), LogLevel::DBG, {}};

    while(true) {
        if (!Paper::Internal::logQueue.try_dequeue(token, threadData)) {
            std::this_thread::yield();
            continue;
        }

        auto const& args = threadData.args;
        auto const& str = threadData.str;
        auto const& tag = threadData.tag;
        auto const& location = threadData.loc;
        auto const& level = threadData.level;
        auto const& time = fmt::localtime(threadData.logTime);
        auto const& threadId = threadData.threadId;

        // "{Ymd} [{HMSf}] {l}[{t:<6}] [{s}]"
        std::string msg(fmt::format(FMT_STRING("{:%Y-%m-%d} [{:%H:%M:%S}] {}[{:<6}] [{}] [{}:{}:{} @ {}]: {}"),
                                    time, time, (int) level, threadId, tag,
                                    location.file_name(), location.line(),
                                    location.column(), location.function_name(),
                                    fmt::vformat(str, args) // TODO: Is there a better way to do this?
        ));

        __android_log_print((int) level, location.file_name().data(), "%s", msg.data());
        globalFile << msg;

        auto it = registeredContexts.find(tag.data());
        if (it != registeredContexts.end()) {
            auto& f = it->second;
            f << msg;
        }
    }
}

void Paper::Logger::Init(std::string_view logPath, std::string_view globalLogFileName) {
    globalLogPath = logPath;
    globalFile.open(fmt::format("{}/{}", logPath, globalLogFileName));
    std::thread(Internal::LogThread).detach();
}

std::string_view Paper::Logger::getLogDirectoryPathGlobal() {
    return globalLogPath;
}

// TODO: Lock?
void Paper::Logger::RegisterContextId(std::string_view contextId, std::string_view logPath) {
    registeredContexts.try_emplace(contextId.data(), fmt::format("{}/{}", getLogDirectoryPathGlobal(), logPath), std::ofstream::out | std::ofstream::trunc);
}

void Paper::Logger::UnregisterContextId(std::string_view contextId) {
    registeredContexts.erase(contextId.data());
}

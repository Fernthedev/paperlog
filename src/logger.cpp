#include "logger.hpp"
#include "main.hpp"

#include <fmt/ostream.h>
#include <fmt/chrono.h>
#include <fmt/compile.h>

#include <optional>
#include <fstream>
#include <iostream>
#include <android/log.h>
#include <thread>
#include <semaphore>
#include <span>

#if __has_include(<unwind.h>)
#include <unwind.h>
#include <cxxabi.h>
#include <unistd.h>
#include <dlfcn.h>

#define HAS_UNWIND
#endif

moodycamel::ConcurrentQueue<Paper::ThreadData> Paper::Internal::logQueue;
std::binary_semaphore flushSemaphore;


#define LINE_END '\n'
#define STRING_LIMIT 1024
#define FORCE_FLUSH_COUNT 500

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

static std::unordered_map<ContextID, LogPath, StringHash, std::equal_to<>> registeredFileContexts;
static LogPath globalFile;

void Paper::Logger::Init(std::string_view logPath, std::string_view globalLogFileName) {
    globalLogPath = logPath;
    globalFile.open(fmt::format("{}/{}", logPath, globalLogFileName));
    std::thread(Internal::LogThread).detach();
}

void logError(std::string_view error) {
    __android_log_print((int) Paper::LogLevel::ERR, "PAPERLOG", "%s", error.data());
    if (globalFile.is_open()) {
        globalFile << error << std::endl;
    }
}

inline void writeLog(Paper::ThreadData const& threadData, std::tm const& time, std::string_view threadId, std::string_view s, /* nullable */ std::ofstream* contextFilePtr) {

    auto const &rawFmtStr = threadData.str;
    auto const &tag = threadData.tag;
    auto const &location = threadData.loc;
    auto const &level = threadData.level;

    // "{Ymd} [{HMSf}] {l}[{t:<6}] [{s}]"
    std::string msg(fmt::format(FMT_COMPILE("{:%Y-%m-%d} [{:%H:%M:%S}] {}[{:<6}] [{}] [{}:{}:{} @ {}]: {}"),
                                time, time, (int) level, threadId, tag,
                                location.file_name(), location.line(),
                                location.column(), location.function_name(),
                                s // TODO: Is there a better way to do this?
    ));

    __android_log_write((int) level, tag.data(),msg.data());
    globalFile << msg << '\n';


    if (contextFilePtr) {
        auto &f = *contextFilePtr;
        f << msg << '\n';
    }
}

[[nodiscard]] constexpr uint8_t charExtraLength(const char c) {
    uint8_t shiftedC = c >> 3;

    if (shiftedC >= 0b11110) {
        return 3;
    }
    if (shiftedC >= 0b11100) {
        return 2;
    }

    if (shiftedC >= 0b11000) {
        return 1;
    }

    return 0;
}

void Paper::Internal::LogThread() {
    try {
        moodycamel::ConsumerToken token(Paper::Internal::logQueue);

        Paper::ThreadData threadData{(std::string_view) "", std::this_thread::get_id(), "", Paper::sl::current(),
                                     LogLevel::DBG, {}};

        std::ofstream* contextFile;
        size_t logsSinceLastFlush = 0;

        while (true) {
            if (!Paper::Internal::logQueue.try_dequeue(token, threadData)) {
                std::this_thread::yield();
                continue;
            }

            auto const &rawFmtStr = threadData.str;
            auto const &tag = threadData.tag;
            auto const &location = threadData.loc;
            auto const &level = threadData.level;
            auto const &time = fmt::localtime(threadData.logTime);
            auto const &threadId = fmt::to_string(threadData.threadId);


            auto it = registeredFileContexts.find(tag);
            if (it != registeredFileContexts.end()) {
                // if new context file, flush immediately
                if (contextFile && &it->second != contextFile) {
                    contextFile->flush();
                }
                contextFile = &it->second;
            } else {
                if (contextFile) {
                    contextFile->flush();
                }
                contextFile = nullptr;
            }

            // Split/chunk string algorithm provided by sc2ad thanks
            // intended for logcat and making \n play nicely
            auto maxStrLength = std::min<size_t>(rawFmtStr.size(), STRING_LIMIT);
            auto begin = rawFmtStr.data();
            std::size_t count = 0;
            uint8_t skipCount = 0;

            for (auto c : rawFmtStr) {
                if (skipCount > 0) {
                    skipCount--;
                    count++;
                    continue;
                }

                if (c == '\n') {
                    writeLog(threadData, time, threadId, std::string_view(begin, count), contextFile);
                    begin += count + 1;
                    count = 0;
                } else if((skipCount = charExtraLength(c)) > 0) {
                    count++;
                } else if (count >= maxStrLength) {
                    writeLog(threadData, time, threadId, std::string_view(begin, count), contextFile);
                    begin += count;
                    count = 1;
                } else {
                    count++;
                }
            }
            if (count > 0) {
                writeLog(threadData, time, threadId, std::string_view(begin, count), contextFile);
            }
            logsSinceLastFlush++;

            auto remainingLogs = Paper::Internal::logQueue.size_approx();
            if (remainingLogs == 0 || (false && logsSinceLastFlush > FORCE_FLUSH_COUNT)) {
                // nothing more in queue, flush
                if (contextFile) {
                    contextFile->flush();
                }
                globalFile.flush();
                logsSinceLastFlush = 0;

                // dont release until we flush ALL logs
                if (remainingLogs == 0) {
                    flushSemaphore.release();
                }
            }
        }
    } catch (std::runtime_error const &e) {
        std::string error = fmt::format("Error occurred in logging thread! %s", e.what());
        logError(error);
        throw e;
    } catch (std::exception const &e) {
        std::string error = fmt::format("Error occurred in logging thread! %s", e.what());
        logError(error);
        throw e;
    } catch (...) {
        std::string error = fmt::format("Error occurred in logging thread!");
        logError(error);
        throw;
    }
}

void Paper::Logger::WaitForFlush() {
    flushSemaphore.acquire();
}

std::string_view Paper::Logger::getLogDirectoryPathGlobal() {
    return globalLogPath;
}

// TODO: Lock?
void Paper::Logger::RegisterFileContextId(std::string_view contextId, std::string_view logPath) {
    registeredFileContexts.try_emplace(contextId.data(), fmt::format("{}/{}.log", getLogDirectoryPathGlobal(), logPath), std::ofstream::out | std::ofstream::trunc);
}

void Paper::Logger::UnregisterFileContextId(std::string_view contextId) {
    registeredFileContexts.erase(contextId.data());
}

#ifdef HAS_UNWIND
// Backtrace stuff largely written by StackDoubleFlow
// https://github.com/sc2ad/beatsaber-hook/blob/138101a5a2b494911583b62140af6acf6e955e72/src/utils/logging.cpp#L211-L289
namespace {
    struct BacktraceState {
        void **current;
        void **end;
    };
    static _Unwind_Reason_Code unwindCallback(struct _Unwind_Context *context, void *arg) {
        BacktraceState *state = static_cast<BacktraceState *>(arg);
        uintptr_t pc = _Unwind_GetIP(context);
        if (pc) {
            if (state->current == state->end) {
                return _URC_END_OF_STACK;
            } else {
                *state->current++ = reinterpret_cast<void *>(pc);
            }
        }
        return _URC_NO_REASON;
    }
    size_t captureBacktrace(void **buffer, uint16_t max) {
        BacktraceState state{buffer, buffer + max};
        _Unwind_Backtrace(unwindCallback, &state);

        return state.current - buffer;
    }
}

void Paper::Logger::Backtrace(std::string_view const tag, uint16_t frameCount) {
    void* buffer[frameCount + 1];
    captureBacktrace(buffer, frameCount + 1);
    fmtLogTag<LogLevel::DBG>("Printing backtrace with: {} max lines:", tag, frameCount);
    fmtLogTag<LogLevel::DBG>("*** *** *** *** *** *** *** *** *** *** *** *** *** *** *** ***", tag);
    fmtLogTag<LogLevel::DBG>("pid: {}, tid: {}", tag, getpid(), gettid());
    for (uint16_t i = 0; i < frameCount; ++i) {
        Dl_info info;
        if (dladdr(buffer[i + 1], &info)) {
            // Buffer points to 1 instruction ahead
            long addr = reinterpret_cast<char*>(buffer[i + 1]) - reinterpret_cast<char*>(info.dli_fbase) - 4;
            if (info.dli_sname) {
                int status;
                const char *demangled = abi::__cxa_demangle(info.dli_sname, nullptr, nullptr, &status);
                if (status) {
                    demangled = info.dli_sname;
                }
                fmtLogTag<LogLevel::DBG>("        #{:02}  pc {:016Lx}  {} ({})", tag, i, addr, info.dli_fname, demangled);
                if (demangled != info.dli_sname) {
                    free(const_cast<char *>(demangled));
                }
            } else {
                fmtLogTag<LogLevel::DBG>("        #{:02}  pc {:016Lx}  {}", tag, i, addr, info.dli_fname);
            }
        }
    }
}

#else

#warning No unwind found, compiling stub backtrace function
void Paper::Logger::Backtrace(std::string_view const tag, uint16_t frameCount) {}

#endif
#include "logger.hpp"

#include <fmt/ostream.h>
#include <fmt/chrono.h>
#include <fmt/compile.h>
#include <fmt/std.h>

#include <exception>
#include <optional>
#include <fstream>
#include <iostream>
#include <thread>
#include <semaphore>
#include <span>
#include <unordered_map>
#include <vector>
#include <filesystem>

#if __has_include(<android/log.h>)
#define PAPERLOG_ANDROID_LOG
#include <android/log.h>
#else
// Stubs
#define ANDROID_LOG_INFO 4
#define ANDROID_LOG_ERROR 6
#endif

// Define this if you want to print to std::cout
// #define PAPERLOG_FMT_C_STDOUT

#if __has_include(<unwind.h>)
#include <unwind.h>
#include <cxxabi.h>
#include <unistd.h>
#include <dlfcn.h>

#define HAS_UNWIND
#endif

moodycamel::ConcurrentQueue<Paper::ThreadData> Paper::Internal::logQueue;
std::binary_semaphore flushSemaphore{1};

struct StringHash {
    using is_transparent = void; // enables heterogenous lookup
    std::size_t operator()(std::string_view sv) const {
        std::hash<std::string_view> hasher;
        return hasher(sv);
    }
};

// Initialize variables before Init runs
#define EARLY_INIT_ATTRIBUTE __attribute__((init_priority(105)));

static Paper::LoggerConfig globalLoggerConfig EARLY_INIT_ATTRIBUTE;

static std::string globalLogPath EARLY_INIT_ATTRIBUTE;

using ContextID = std::string;
using LogPath = std::ofstream;

static std::vector<Paper::LogSink> sinks EARLY_INIT_ATTRIBUTE;
static std::unordered_map<ContextID, LogPath, StringHash, std::equal_to<>> registeredFileContexts EARLY_INIT_ATTRIBUTE;

static LogPath globalFile EARLY_INIT_ATTRIBUTE;

constexpr auto globalFileName = "PaperLog.log";

// To avoid loading errors
static bool inited = false;

template <typename... TArgs>
inline void WriteStdOut(int level, std::string_view ctx, std::string_view s) {
#ifdef PAPERLOG_ANDROID_LOG
    __android_log_write(
        level, ctx.data(),
        s.data());
#endif

#ifdef PAPERLOG_FMT_C_STDOUT
#warning Printing to stdout
#ifndef PAPERLOG_FMT_NO_PREFIX
    fmt::print(std::cout, "Level ({}) [{}] ", level, ctx);
#endif
    fmt::println(std::cout, fmt::runtime(s), std::forward<TArgs>(args)...);
#endif
}

namespace Paper::Logger {
    void Init(std::string_view logPath, LoggerConfig const &config) {
        if (inited) {
            throw std::runtime_error("Already started the logger thread!");
        }

        WriteStdOut(ANDROID_LOG_INFO, "PAPERLOG",
                       "Logging paper to folder " + std::string(logPath) + "and file " + globalFileName);

        globalLoggerConfig = {config};
        globalLogPath = logPath;
        std::filesystem::create_directories(globalLogPath);
        globalFile.open(fmt::format("{}/{}", logPath, globalFileName), std::ofstream::out | std::ofstream::trunc);
        std::thread(Internal::LogThread).detach();
        flushSemaphore.release();
        inited = true;
    }

    bool IsInited() {
        return inited;
    }
}

// TODO: Fix constructor memory crash
#ifdef PAPER_NO_INIT

#warning Using dlopen for initializing thread
void __attribute__((constructor(200))) dlopen_initialize() {
        WriteStdOut(ANDROID_LOG_INFO, "PAPERLOG", "DLOpen initializing");

#ifdef PAPER_QUEST_MODLOADER
    std::string path = fmt::format("/sdcard/Android/data/{}/files/logs/paper", Modloader::getApplicationId());
#else
    #warning "Must have a definition for globalLogPath if PAPER_NO_INIT is defined!
    std::string path(globalLogPath);
#endif
    try {
        Paper::Logger::Init(path, Paper::LoggerConfig());
    } catch (std::exception const &e) {
        std::string error = fmt::format("Error occurred in logging thread! {}", e.what());
        WriteStdOut(ANDROID_LOG_ERROR, "PAPERLOG", error.data());
        throw e;
    } catch (...) {
        std::string error = fmt::format("Error occurred in logging thread!");
        WriteStdOut(ANDROID_LOG_ERROR, "PAPERLOG", error.data());
        throw;
    }
}
#endif

Paper::LoggerConfig& GlobalConfig() {
    return globalLoggerConfig;
}

inline void logError(std::string_view error) {
    WriteStdOut((int)Paper::LogLevel::ERR, "PAPERLOG", error);
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
#ifndef PAPERLOG_FMT_NO_PREFIX
    std::string const msg(fmt::format(FMT_COMPILE("{:%Y-%m-%d} [{:%H:%M:%S}] {}[{:<6}] [{}] [{}:{}:{} @ {}]: {}"),
                                time, time, level, threadId, tag,
                                location.file_name(), location.line(),
                                location.column(), location.function_name(),
                                s // TODO: Is there a better way to do this?
    ));

    // TODO: Reduce double formatting
    std::string_view locationFileName(location.file_name());

    std::string androidMsg(fmt::format(FMT_COMPILE("{}[{:<6}] [{}:{}:{} @ {}]: {}"),
                                             level, threadId,
                                             locationFileName
                                             // don't allow file name to be super long
                                                .substr(std::min<size_t>(locationFileName.size() - globalLoggerConfig.MaximumFileLengthInLogcat, 0)),
                                             location.line(),
                                             location.column(), location.function_name(),
                                             s));
#else
    std::string const msg(s);
    std::string const androidMsg(s);
#endif

    WriteStdOut((int)level, tag.data(), androidMsg.data());
    globalFile << msg << '\n';

    if (contextFilePtr) {
        auto &f = *contextFilePtr;
        f << msg << '\n';
    }

    for (auto const& sink : sinks) {
        sink(threadData, msg);
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

        std::ofstream* contextFile = nullptr;
        size_t logsSinceLastFlush = 0;

        bool doFlush = false;
        auto flushLambda = [&]() constexpr {
            // nothing more in queue, flush
            if (contextFile) {
                contextFile->flush();
            }
            globalFile.flush();
            logsSinceLastFlush = 0;
            doFlush = false;

            flushSemaphore.release();
        };

        // Write to log to empty
        flushLambda();

        while (true) {
            if (!Paper::Internal::logQueue.try_dequeue(token, threadData)) {
                if (doFlush) {
                    flushLambda();
                }
                flushSemaphore.release();
                std::this_thread::yield();
                std::this_thread::sleep_for(std::chrono::microseconds(100));
                continue;
            }

            auto const &rawFmtStr = threadData.str;
            auto const &tag = threadData.tag;
            auto const &location = threadData.loc;
            auto const &level = threadData.level;
            auto const &systemTime =
                std::chrono::system_clock::to_time_t(threadData.logTime);
            auto const &time = fmt::localtime(systemTime);
            std::string threadId = fmt::to_string(threadData.threadId);

            auto writeLogLambda = [&](std::string_view view) constexpr {
                writeLog(threadData, time, threadId, view, contextFile);
                doFlush = true;
            };

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
            auto maxStrLength = std::min<size_t>(rawFmtStr.size(), globalLoggerConfig.MaxStringLen);
            auto begin = rawFmtStr.data();
            std::size_t stringEndOffset = 0;
            uint8_t skipCount = 0;


            for (auto c : rawFmtStr) {
                if (skipCount > 0) {
                    skipCount--;
                    stringEndOffset++;
                    continue;
                }

                // line break, write and continue
                if (c == globalLoggerConfig.lineEnd) {
                    //  TODO: string view length not being respected in Clang 15
                    //  Linux when writing to std::cout
                    // causing line break to not work
                    writeLogLambda(std::string_view(begin, stringEndOffset));
                    begin += stringEndOffset + 1;
                    stringEndOffset = 0;
                // skipping extra bytes because utf8 is variable
                } else if((skipCount = charExtraLength(c)) > 0) {
                    stringEndOffset++;
                // string reached max length
                } else if (stringEndOffset >= maxStrLength) {
                    writeLogLambda(std::string_view(begin, stringEndOffset));
                    begin += stringEndOffset;
                    stringEndOffset = 1;
                // increment string end index
                } else {
                    stringEndOffset++;
                }
            }
            // Write remaining string contents
            if (stringEndOffset > 0) {
                writeLogLambda(std::string(begin, stringEndOffset));
            }
            logsSinceLastFlush++;

            // Since I completely forgot what happened here
            // This commit suggests it reduces latency
            // https://github.com/Fernthedev/paperlog/commit/931b15a7f5b494272b486acabc3062038db79fa1#diff-2c46dd80094c3ffd00cd309628cb1d6e5c695f69f8dafb5c40747369a5d6ded0R199
            if (false && logsSinceLastFlush > globalLoggerConfig.LogMaxBufferCount) {
                flushLambda();
            }
        }
    } catch (std::exception const &e) {
        std::string error = fmt::format("Error occurred in logging thread! {}", e.what());
        logError(error);
        #ifndef PAPER_NO_INIT
        inited = false;
        #endif
        throw e;
    } catch (...) {
        std::string error = fmt::format("Error occurred in logging thread!");
        logError(error);
        #ifndef PAPER_NO_INIT
        inited = false;
        #endif
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

void
Paper::Logger::AddLogSink(LogSink const& sink) {
    sinks.emplace_back(sink);
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
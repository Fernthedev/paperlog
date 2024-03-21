#pragma once

#include "internal_logger.hpp"
#include "logger.hpp"
#include <fmt/compile.h>

#ifdef PAPERLOG_FMT_NO_PREFIX
#warning Removing fmt prefixes from logs
#endif

#ifdef PAPERLOG_STDOUT_LOG
#warning Logging to stdout!
#endif

#ifdef PAPERLOG_ANDROID_LOG
#warning Logging to android logcat!
#include <android/log.h>
#endif


namespace Paper::Sinks {

/// Method used to log internal Paper logs
inline void logInternal(Paper::LogLevel level, std::string_view s) {

// TODO: Global File
// TODO: Context File
  
#ifdef PAPERLOG_ANDROID_LOG
  __android_log_write(static_cast<int>(level), "PAPER", s.data());
#endif

#ifdef PAPERLOG_STDOUT_LOG
  std::cout << "PAPER: [" << Paper::format_as(level) << "] " << s << std::endl;
#endif
}

inline void logError(std::string_view error) {
  Paper::Sinks::logInternal(Paper::LogLevel::ERR, error);
}


#ifdef PAPERLOG_STDOUT_LOG
inline void stdOutSink(Paper::LogData const& threadData, std::string_view fmtMessage,
                       std::string_view _unformattedMessage) {
  auto const& level = threadData.level;
  auto const& tag = threadData.tag;

  std::cout << "Level (" << Paper::format_as(level) << ") ";
  std::cout << "[" << tag << "] ";
  std::cout << fmtMessage << '\n';
}
#endif

#ifdef PAPERLOG_ANDROID_LOG
inline void androidLogcatSink(Paper::LogData const& threadData, std::string_view _fmtMessage,
                              std::string_view unformattedMessage) {
  auto const& level = threadData.level;
  auto const& tag = threadData.tag;

  // Reduce log bloat for android logcat
#ifdef PAPERLOG_FMT_NO_PREFIX
  auto androidMsg = unformattedMessage;
#else
  auto const& location = threadData.loc;
  auto const& threadId = fmt::to_string(threadData.threadId);

  // TODO: Reduce double formatting
  std::string_view locationFileName(location.file_name());
  auto maxLen = std::min<size_t>(locationFileName.size() - Paper::Logger::GlobalConfig().MaximumFileLengthInLogcat, 0);

  // limit length
  // don't allow file name to be super long
  locationFileName = locationFileName.substr(maxLen);

  // custom format because logcat already stores other information
  std::string androidMsg(fmt::format(FMT_COMPILE("{}[{:<6}] [{}:{}:{} @ {}]: {}"), level, threadId, locationFileName,
                                     location.line(), location.column(), location.function_name(), unformattedMessage));

#endif
  __android_log_write(static_cast<int>(level), tag.data(), androidMsg.data());
}
#endif

} // namespace Paper::Sinks

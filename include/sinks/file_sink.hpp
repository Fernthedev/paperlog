#pragma once

#include "internal_logger.hpp"
#include <fstream>

#ifdef PAPERLOG_GLOBAL_FILE_LOG
#define PAPERLOG_FILE_LOG
// TODO: I don't remember why I meant it doesn't fully comply
#warning Logging to global file log! Does not fully comply at the moment
#endif

#ifdef PAPERLOG_CONTEXT_FILE_LOG
#define PAPERLOG_FILE_LOG
#warning Logging to context file log! Does not fully comply at the moment
#endif

#ifdef PAPERLOG_FILE_LOG
namespace Paper::Sinks::File {

#ifdef PAPERLOG_CONTEXT_FILE_LOG
void contextFileSink(Paper::LogData const& threadData, std::string_view fmtMessage, std::string_view unformattedMessage,
                     /* nullable */ std::ofstream* contextFilePtr);
#endif

#ifdef PAPERLOG_GLOBAL_FILE_LOG
void globalFileSink(Paper::LogData const& threadData, std::string_view fmtMessage, std::string_view unformattedMessage,
                    /* nullable */ std::ofstream* contextFilePtr);
#endif
} // namespace Paper::Sinks::File
#endif
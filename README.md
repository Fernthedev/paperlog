# Paper

Logging lightweight as paper

## Done:
- Ideally source location
- Template log calls
- Backtrace logging
- Better consumer thread usage for logging to file (?)
- Print `\n` nicely
- Ensure support for logging ustrings

## TODO:
- Flush needs to happen before crash (perhaps signal handler?, Less important)
- Support for logging things that are c# instances by logging pointer value then calling to string (in bs-hooks)


## Get Started
Paper makes use of [fmt](https://fmt.dev/latest/api.html), which has many optional formatters and features that are supported out of the box.

By default, Paper will log write all logs in global context (which includes the global log file)
`/sdcard/Android/data/com.beatgames.beatsaber/files/logs/paper/PaperLog.log`

The basics are straightforward: 
```cpp
#include "paper/shared/logger.hpp"

Paper::Logger::fmtLog<Paper::LogLevel::INF>("hi! {}", 5); // hi 5
Paper::Logger::fmtLog<Paper::LogLevel::WRN>("Another log! {1} name {0}", "foo", "my"); // Another log! my name foo

Paper::Logger::fmtThrowError<std::runtime_error>("Some error {}", "bad data"); // Throws an exception and logs before
```

### Contexts (and custom file logging)
To tag your logs and have a filtered log file. The context can be stored in static memory; that is to say, outside a function and instance field declaration.
```cpp
// by default, it creates a log file specific to `PaperFast` logs,
// named `PaperFast.log` in the same directory
auto fastContext = Paper::Logger::WithContext<"PaperFast">();

fastContext.fmtLog<Paper::LogLevel::INF>("Paper is so cool!");

auto noFileContext = Paper::Logger::WithContext<"NoFile", false>();
noFileContext.fmtLog<Paper::LogLevel::INF>("Only logs to logcat");
```

You can manually log a context or register a context to log to files with the following 
```cpp
Paper::Logger::RegisterFileContextId("context", "log file path relative to log folder")

Paper::Logger::fmtLogTag("log", "context", args...);
```

### Backtraces 
```cpp
Paper::Logger::Backtrace(20);
myContext.Backtrace(20);
```

### Profiler
Paper includes a rudimentary basic profiler which you can use to measure the latency of certain areas of your program.
```cpp
Paper::Profiler<std::chrono::nanoseconds> profiler;
profiler.suffix = "ns";

someExpensiveComputation();
profiler.mark("Expensive computation 1");
someExpensiveComputation2();
profiler.mark("Expensive computation 1");
profiler.printMarks();
```

### Sinks
Paper supports sinks. This one's for you Laurie
```cpp
Paper::Logger::AddLogSink([](ThreadData const& data, std::string_view fmtMessage) {
    
});
```
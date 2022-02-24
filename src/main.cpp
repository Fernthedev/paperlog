#include "main.hpp"

#include "modinfo_fmt.hpp"
#include "logger.hpp"

#include "beatsaber-hook/shared/utils/logging.hpp"

static ModInfo modInfo; // Stores the ID and version of our mod, and is sent to the modloader upon startup


// Called at the early stages of game loading
extern "C" void setup(ModInfo& info) {
    info.id = MOD_ID;
    info.version = VERSION;
    modInfo = info;

}

Logger& getLogger() {
    static auto* logger = new Logger(modInfo, LoggerOptions(false, true));
    return *logger;
}

// Called later on in the game loading - a good time to install function hooks
extern "C" void load() {
    try {
        getLogger().debug("Init");
        Paper::Logger::Init("/sdcard/Android/data/com.beatgames.beatsaber/files/logs");

        getLogger().debug("Logging test!");
        Paper::Logger::fmtLog<fmtlog::LogLevel::INF>("hi! {}", 5);
        Paper::Logger::fmtLog<fmtlog::LogLevel::INF>("Paper loaded! {}", modInfo);

//        // creates a file logger
        auto context = Paper::Logger::getModloaderContext(modInfo);
        context.fmtLog<fmtlog::LogLevel::INF>("yo!");


    } catch (std::exception const& e) {
        getLogger().error("Crash %s", e.what());
        throw e;
    } catch (fmt::format_error const& e) {
        getLogger().error("Crash %s", e.what());
    }
}
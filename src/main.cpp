#include "main.hpp"

// Defines HAS_QUEST_MODLOADER
#include "modinfo_fmt.hpp"

#include "Tests.hpp"


static ModInfo modInfo; // Stores the ID and version of our mod, and is sent to the modloader upon startup

#ifdef HAS_QUEST_MODLOADER
// Called at the early stages of game loading
extern "C" void setup(ModInfo& info) {
    info.id = MOD_ID;
    info.version = VERSION;
    modInfo = info;

}

// Called later on in the game loading - a good time to install function hooks
extern "C" void load() {
    Paper::Test::runTests();
}

#endif

int main() {
    Paper::Test::runTests();
    std::terminate();
}
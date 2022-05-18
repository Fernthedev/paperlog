#pragma once

#if __has_include("modloader/shared/modloader.hpp")
#define PAPER_QUEST_MODLOADER
#endif

#ifdef PAPER_QUEST_MODLOADER
#define PAPER_NO_INIT
// TODO: Add some more potentially specific includes here
#endif

#pragma once

#if __has_include("modloader/shared/modloader.hpp") && defined(ANDROID)
#define PAPER_QUEST_MODLOADER
#endif

#ifdef NO_PAPER_QUEST_MODLOADER
#undef PAPER_QUEST_MODLOADER
#warning Removing quest modloader dependency
#endif

#ifdef PAPER_QUEST_MODLOADER
#define PAPER_NO_INIT
// TODO: Add some more potentially specific includes here
#endif

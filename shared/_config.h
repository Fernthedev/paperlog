#pragma once

#if __has_include("modloader/shared/modloader.hpp") && defined(ANDROID)
#define PAPER_QUEST_MODLOADER
#endif
#if __has_include("scotland2/shared/modloader.h") && defined(ANDROID)
#define PAPER_QUEST_SCOTLAND2
#endif

#ifdef NO_PAPER_QUEST_MODLOADER
#undef PAPER_QUEST_MODLOADER
#warning Removing quest modloader dependency
#endif
#ifdef NO_PAPER_QUEST_SCOTLAND2
#undef PAPER_QUEST_SCOTLAND2
#warning Removing quest modloader dependency
#endif

#define PAPER_EXPORT __attribute__((visibility("default")))
#ifdef __cplusplus
#define PAPER_FUNC extern "C" __attribute__((visibility("default")))
#else
#define PAPER_FUNC MODLOADER_EXPORT
#endif

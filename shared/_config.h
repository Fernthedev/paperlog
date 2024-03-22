#pragma once

#if __has_include("modloader/shared/modloader.hpp") && defined(ANDROID)
#define PAPER_QUEST_MODLOADER
#endif
#if __has_include("scotland2/shared/modloader.h") && defined(ANDROID)
#define PAPER_QUEST_SCOTLAND2
#endif

#ifdef NO_PAPER_QUEST_MODLOADER
#undef PAPER_QUEST_MODLOADER
#ifdef __GNUC__
#warning(Removing quest modloader dependency)
#else
#pragma NOTE(Removing quest modloader dependency)
#endif
#endif
#ifdef NO_PAPER_QUEST_SCOTLAND2
#undef PAPER_QUEST_SCOTLAND2
#ifdef __GNUC__
#warning(Removing quest modloader dependency)
#else
#pragma NOTE(Removing quest modloader dependency)
#endif
#endif

// Generic helper definitions for shared library support
#if defined _WIN32 || defined __CYGWIN__
#define PAPER_IMPORT __declspec(dllimport)
#define PAPER_EXPORT __declspec(dllexport)
#define PAPER_LOCAL
#else
#if __GNUC__ >= 4
#define PAPER_IMPORT __attribute__((visibility("default")))
#define PAPER_EXPORT __attribute__((visibility("default")))
#define PAPER_LOCAL __attribute__((visibility("hidden")))
#else
#define PAPER_IMPORT
#define PAPER_EXPORT
#define PAPER_LOCAL
#endif
#endif

#ifdef __cplusplus
#define PAPER_FUNC extern "C" PAPER_EXPORT
#else
#define PAPER_FUNC MODLOADER_EXPORT
#endif

#pragma once

#if __has_include(<android/log.h>)
#define PAPERLOG_ANDROID_LOG
#include <android/log.h>
#else
// Stubs
#define ANDROID_LOG_INFO 4
#define ANDROID_LOG_ERROR 6
#endif
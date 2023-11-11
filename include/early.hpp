#pragma once

// Initialize variables before Init runs
#define EARLY_INIT_ATTRIBUTE __attribute__((init_priority(105)))
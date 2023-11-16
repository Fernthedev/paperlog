# include some defines automatically made by qpm
include(./cmake/qpm.cmake)

# override mod id
set(MOD_ID "paper")

# Enable link time optimization
# In my experience, this can be highly unstable but it nets a huge size optimization and likely performance
# However, the instability was seen using Android.mk/ndk-build builds. With Ninja + CMake, this problem seems to have been solved.
# As always, test thoroughly
# - Fern
set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)

cmake_minimum_required(VERSION 3.21)
project(paperlog_project)

# c++ standard
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED 20)

set(CMAKE_CXX_EXTENSIONS OFF)

# Output everything to the same folder
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/build)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/build)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/build)

# set(CMAKE_CXX_STANDARD_LIBRARIES "${CMAKE_CXX_STANDARD_LIBRARIES} -static-libstdc++")

# define that stores the actual source directory
set(SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src)
set(INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/include)

# compile options used
add_compile_options(-frtti -fexceptions)
add_compile_options(-O3)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# compile definitions used
add_compile_definitions(VERSION=\"${MOD_VERSION}\")
add_compile_definitions(MOD_ID=\"${MOD_ID}\")
add_compile_definitions(FMT_HEADER_ONLY)

if(DEFINED TEST_BUILD)
        MESSAGE(STATUS "Compiling with test defines")
        add_compile_definitions(PAPER_TEST_LOGS)
endif()

# Add to all
include_directories(SYSTEM ${EXTERN_DIR}/includes/fmt/fmt/include/)
add_compile_options(-DFMT_HEADER_ONLY)
add_compile_options(-flto)

if(NOT DEFINED QPM_ANDROID)
        include(${CMAKE_CURRENT_LIST_DIR}/cmake/gtest.cmake)
endif()

add_subdirectory(${SOURCE_DIR})
add_subdirectory(src_bootstrapper)

cmake_minimum_required(VERSION 3.22)

option(QUEST "Build for quest" ON)
option(TEST "Build tests" OFF)

# Globals
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED 20)
set(CMAKE_EXPORT_COMPILE_COMMANDS TRUE)

set(SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src)
set(INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/include)
set(EXTERN_DIR ${CMAKE_CURRENT_SOURCE_DIR}/extern)

# stop symbols leaking
# add_link_options(-Wl, --exclude-libs, ALL)
add_compile_options(-frtti -fexceptions -fvisibility=hidden -fPIE -fPIC)

# Add to all
include_directories(SYSTEM ${EXTERN_DIR}/includes/fmt/fmt/include/)
add_compile_definitions(FMT_HEADER_ONLY)
include_directories(SYSTEM ${CMAKE_CURRENT_SOURCE_DIR}/shared/utfcpp/source)

# Add includes
if(TEST)
        include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/gtest.cmake)
endif()

# Quest
if(QUEST)
        include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/targets/quest.cmake)
else()
        include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/targets/desktop.cmake)
endif()

# Build options must be set at end
include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/utils.cmake)
include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/qpm.cmake)

include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/builds/release.cmake)
include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/builds/debug.cmake)
include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/builds/strip.cmake)

project(paperlog
        VERSION ${PACKAGE_VERSION})

add_compile_definitions(MOD_ID="${CMAKE_PROJECT_NAME}")
add_compile_definitions(VERSION="${CMAKE_PROJECT_VERSION}")

# Set COMPILE_ID for qpm extern purposes
set(COMPILE_ID ${CMAKE_PROJECT_NAME})

# recursively get all src files
RECURSE_FILES(cpp_file_list ${SOURCE_DIR}/*.cpp)
RECURSE_FILES(c_file_list ${SOURCE_DIR}/*.c)

add_library(
        ${CMAKE_PROJECT_NAME}
        SHARED
        ${cpp_file_list}
        ${c_file_list}
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${INCLUDE_DIR})
target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${SHARED_DIR})

# TODO: Setup qpm extern from toolchain
# Must be set at end
if(DEFINED ANDROID)
        target_link_libraries(${COMPILE_ID} PRIVATE -llog)
endif()

add_subdirectory(src_bootstrapper)
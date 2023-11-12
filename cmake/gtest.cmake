message("Compiling with GTest")

add_compile_definitions(PAPERLOG_FMT_C_STDOUT)
add_compile_definitions(PAPERLOG_FMT_NO_PREFIX)

# GTest
include(FetchContent)
FetchContent_Declare(
    googletest
    URL https://github.com/google/googletest/archive/03597a01ee50ed33e9dfd640b249b4be3799d395.zip
)

# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

enable_testing()

add_executable(
    paperlog_test
    test/log.cpp
    test/utf.cpp
)
target_link_libraries(
    paperlog_test
    GTest::gtest_main
    ${COMPILE_ID}
)

# add src dir as include dir
target_include_directories(paperlog_test PRIVATE test)

# add include dir as include dir
target_include_directories(paperlog_test PRIVATE ${INCLUDE_DIR})

# add shared dir as include dir
target_include_directories(paperlog_test SYSTEM PUBLIC ${SHARED_DIR})
target_include_directories(paperlog_test SYSTEM PUBLIC extern/includes)
target_include_directories(paperlog_test SYSTEM PUBLIC extern/includes/fmt/fmt/include/)

include(GoogleTest)
gtest_discover_tests(paperlog_test)
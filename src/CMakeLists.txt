# project(${COMPILE_ID})

# recursively get all src files
RECURSE_FILES(cpp_file_list ${SOURCE_DIR}/*.cpp)
RECURSE_FILES(c_file_list ${SOURCE_DIR}/*.c)

# add all src files to compile
add_library(
    ${COMPILE_ID}
    SHARED
    ${cpp_file_list}
    ${c_file_list}
)

target_include_directories(${COMPILE_ID} PRIVATE .)

# add src dir as include dir
target_include_directories(${COMPILE_ID} PRIVATE ${SOURCE_DIR})

# add include dir as include dir
target_include_directories(${COMPILE_ID} PRIVATE ${INCLUDE_DIR})

# add shared dir as include dir
target_include_directories(${COMPILE_ID} PUBLIC ${SHARED_DIR})

# logcat
if(DEFINED QPM_ANDROID)
    target_link_libraries(${COMPILE_ID} PRIVATE -llog)

    # add extern stuff like libs and other includes
    include(${CMAKE_SOURCE_DIR}/extern.cmake)
endif()

add_custom_command(TARGET ${COMPILE_ID} POST_BUILD
    COMMAND ${CMAKE_STRIP} -d --strip-all
    "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/lib${COMPILE_ID}.so" -o "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/stripped_lib${COMPILE_ID}.so"
    COMMENT "Strip debug symbols done on final binary.")

add_custom_command(TARGET ${COMPILE_ID} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/debug
    COMMENT "Make directory for debug symbols"
)

add_custom_command(TARGET ${COMPILE_ID} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E rename ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/lib${COMPILE_ID}.so ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/debug/lib${COMPILE_ID}.so
    COMMENT "Rename the lib to debug_ since it has debug symbols"
)

add_custom_command(TARGET ${COMPILE_ID} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E rename ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/stripped_lib${COMPILE_ID}.so ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/lib${COMPILE_ID}.so
    COMMENT "Rename the stripped lib to regular"
)
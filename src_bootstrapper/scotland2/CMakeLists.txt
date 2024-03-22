# project(paperlog_sl2)
add_library(
    paperlog_sl2
    SHARED
    ${CMAKE_CURRENT_LIST_DIR}/scotland2.cpp
)

if (DEFINED ANDROID)
    target_link_libraries(paperlog_sl2 -llog)
endif()

message("Including ${INCLUDE_DIR}")

target_include_directories(paperlog_sl2 PRIVATE ${INCLUDE_DIR})
target_include_directories(paperlog_sl2 PRIVATE ${SHARED_DIR})
target_include_directories(paperlog_sl2 PRIVATE ${CMAKE_CURRENT_LIST_DIR}/extern/includes/)

# target_include_directories(paperlog_sl2 PRIVATE ../../extern/includes/fmt)
target_link_libraries(
    paperlog_sl2 
    ${COMPILE_ID}
)

# target_link_directories(paperlog_sl2 PRIVATE ${CMAKE_CURRENT_LIST_DIR}/extern/libs)
target_link_libraries(paperlog_sl2 ${CMAKE_CURRENT_LIST_DIR}/extern/libs/libsl2.so)
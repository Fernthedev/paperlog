set(MOD_VERSION "0.1.0")

# derived from override .so name or just id_version
set(COMPILE_ID "paperlog")

# given from qpm, automatically updated from qpm.json
set(EXTERN_DIR_NAME "extern")
set(SHARED_DIR_NAME "shared")

# if no target given, use RelWithDebInfo
if (NOT DEFINED CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE "RelWithDebInfo")
endif()

# define used for external data, mostly just the qpm dependencies
set(EXTERN_DIR ${CMAKE_CURRENT_SOURCE_DIR}/${EXTERN_DIR_NAME})
set(SHARED_DIR ${CMAKE_CURRENT_SOURCE_DIR}/${SHARED_DIR_NAME})
# get files by filter recursively
MACRO(RECURSE_FILES return_list filter)
    FILE(GLOB_RECURSE new_list ${filter})
    SET(file_list "")
    FOREACH(file_path ${new_list})
        SET(file_list ${file_list} ${file_path})
    ENDFOREACH()
    LIST(REMOVE_DUPLICATES file_list)
    SET(${return_list} ${file_list})
ENDMACRO()


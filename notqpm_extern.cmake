# always added
target_include_directories(${COMPILE_ID} PRIVATE ${EXTERN_DIR}/includes)

# includes and compile options added by other libraries
target_include_directories(${COMPILE_ID} SYSTEM PRIVATE ${EXTERN_DIR}/includes/fmt/fmt/include/)
target_compile_options(${COMPILE_ID} PRIVATE -DFMT_HEADER_ONLY)

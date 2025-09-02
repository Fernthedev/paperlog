#!/bin/bash
cargo build --manifest-path ../Cargo.toml --features stdout,tracing
ls -la ../target/debug

clang++ -c ./test.cpp -o test.o -std=c++20 \
  -isystem ../shared \
  -isystem ../shared/utfcpp/source \
  -isystem ../extern/includes/fmt/fmt/include/ \
  -DFMT_HEADER_ONLY=true


clang++ test.o -o test -L ../target/debug/ -l paper2
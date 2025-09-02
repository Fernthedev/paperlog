#!/bin/bash
cargo build --manifest-path ../Cargo.toml

clang++ -c ./test.cpp -o test.o -std=c++20 \
  -isystem ../shared \
  -isystem ../shared/utfcpp/source \
  -isystem ../extern/includes/fmt/fmt/include/ 


clang++ test.o -o test   -L ../target/debug/ -l paper2
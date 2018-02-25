#!/bin/bash

set -x
rm -rf ./build

rustup update nightly
rustup override add nightly
rustup component add rust-src

rust_dir=$(rustc --print sysroot)/lib/rustlib/src/rust

git submodule update


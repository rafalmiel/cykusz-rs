#!/usr/bin/bash

git submodule update --init --recursive

./sysroot/make_docker_image.sh
./sysroot/toolchain_docker.sh
./sysroot/toolchain_docker_cross_llvm.sh
./sysroot/toolchain_docker_cross_rust.sh

rustup override set nightly
rustup component add rust-src

./disk-scripts/create_disk.sh

make

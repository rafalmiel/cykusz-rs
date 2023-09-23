#!/bin/bash

cargo fix -p cykusz-rs -p user-alloc -p syscall-user -p syscall-defs --lib --allow-dirty --allow-staged
cargo fmt

#!/bin/bash

cargo fix -p cykusz_rs -p user_alloc -p syscall-user -p syscall-defs --lib --allow-dirty --allow-staged
cargo fmt

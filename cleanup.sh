#!/bin/bash

cargo fix -p cykusz_rs -p user_alloc --lib --allow-dirty --allow-staged
cargo fmt

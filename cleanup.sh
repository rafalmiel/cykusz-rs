#!/bin/bash

cargo fix -p cykusz_rs --lib --allow-dirty --allow-staged
cargo fmt

[![Build Status](https://travis-ci.com/rafalmiel/cykusz-rs.svg?branch=master)](https://travis-ci.com/rafalmiel/cykusz-rs)

# cykusz-rs

## Intro

cykusz-rs is a toy operating system written in Rust programming language.

## Features

Currently implemented:

- [x] 64bit higher-half kernel
- [x] VGA text output
- [x] Physical memory allocator
- [x] Paging
- [x] Kernel heap
- [x] Interrupt handlers
- [x] Partial ACPI support (ioapic, lapic)
- [x] SMP
- [x] Per-CPU-Storage using thread_local
- [x] Preemptive per-cpu scheduler
- [ ] PS2/Keyboard driver + basic TTY (wip)
- [ ] Virtual File System (wip)
- [ ] Userspace support (wip)

## Building

You will need following packages to compile and run the os:
* rust ([rustup](https://rustup.rs/))
* nasm
* qemu

Steps:
```bash
rustup override set nightly
rustup component add rust-src
cargo install xargo
make

make run
```

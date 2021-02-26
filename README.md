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
- [x] ACPI support (ioapic, lapic, acpica)
- [x] SMP
- [x] Per-CPU-Storage using thread_local
- [x] Preemptive per-cpu scheduler
- [x] PS2/Keyboard driver + basic TTY
- [x] Virtual File System
- [x] Storage (ide, ahci)
- [x] Filesystem (ext2)
- [x] Network Stack (e1000, ETH, ARP, IP, UDP, TCP, ICMP, DHCP, DNS)
- [x] Userspace support

## Building

You will need following packages to compile and run the os:
* rust ([rustup](https://rustup.rs/))
* nasm
* qemu

Building:
```bash
git clone https://github.com/rafalmiel/cykusz-rs.git
git submodule update --init --recursive

rustup override set nightly
rustup component add rust-src
cargo install xargo
make

./create_disk.sh
```

Running on qemu:
```bash
make run
```

Running on bochs:
```bash
make bochs
```

Running on VirtualBox:
```bash
# Run only once to import the image into VirtualBox
./create_vbox_image.sh

make vbox
```


[![CI](https://github.com/rafalmiel/cykusz-rs/actions/workflows/main.yml/badge.svg)](https://github.com/rafalmiel/cykusz-rs/actions/workflows/main.yml) [![CI](https://github.com/rafalmiel/cykusz-rs/actions/workflows/toolchain.yml/badge.svg)](https://github.com/rafalmiel/cykusz-rs/actions/workflows/toolchain.yml)

# cykusz-rs

## Intro

cykusz-rs is a toy operating system written in Rust programming language.

https://user-images.githubusercontent.com/3881998/121915370-0ec4d300-cd2b-11eb-9581-9981bc44ea40.mov

## Kernel

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
- [x] PS2/Keyboard driver
- [x] Tty with ansi escape codes
- [x] Virtual File System
- [x] Page/Inode/Directory cache
- [x] Memory mapped files (mmap interface)
- [x] Pipes
- [x] Storage (ide, ahci)
- [x] Filesystem (ext2)
- [x] Network Stack (e1000, ETH, ARP, IP, UDP, TCP, ICMP, DHCP, DNS)

## Userspace

- [x] Basic shell
- [x] libc (mlibc port)
- [x] Exec/fork
- [x] Threads
- [x] Thread local storage
- [x] Fs mount/umount
- [x] Posix signals
- [x] Futexes
- [x] Shared libs

## Ports

- [x] mlibc (<https://github.com/rafalmiel/mlibc/tree/cykusz>) ([upstream](https://github.com/managarm/mlibc))
- [x] binutils (<https://github.com/rafalmiel/binutils-gdb/tree/cykusz>)
- [x] gcc (<https://github.com/rafalmiel/gcc/tree/cykusz>)
- [x] nyancat (<https://github.com/rafalmiel/nyancat/tree/cykusz>)

## Building

You will need following packages to compile and run the os:
* rust ([rustup](https://rustup.rs/))
* nasm
* qemu
* grub2
* parted

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


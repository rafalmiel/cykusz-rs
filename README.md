[![CI](https://github.com/rafalmiel/cykusz-rs/actions/workflows/main.yml/badge.svg)](https://github.com/rafalmiel/cykusz-rs/actions/workflows/main.yml) [![CI](https://github.com/rafalmiel/cykusz-rs/actions/workflows/toolchain.yml/badge.svg)](https://github.com/rafalmiel/cykusz-rs/actions/workflows/toolchain.yml)

# cykusz-rs

## Intro

cykusz-rs is a toy operating system written in Rust programming language.

https://github.com/rafalmiel/cykusz-rs/assets/3881998/afa514a1-f435-4eeb-8c80-200129e6900d.mov

## Kernel

- [x] 64bit higher-half kernel
- [x] VESA fb output
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

### Libs
- [x] mlibc (<https://github.com/rafalmiel/mlibc/tree/cykusz>) ([upstream](https://github.com/managarm/mlibc))
- [x] ncurses (<https://github.com/rafalmiel/ncurses/tree/cykusz>)
- [x] readline (<https://github.com/rafalmiel/readline/tree/cykusz>)
- [x] zlib (<https://github.com/rafalmiel/zlib/tree/cykusz>)
- [x] libressl (<https://github.com/rafalmiel/libressl-portable/tree/cykusz>)
- [x] libpsl (<https://github.com/rafalmiel/libpsl/tree/cykusz>)
- [x] pcre2 (<https://github.com/rafalmiel/pcre2/tree/cykusz>)
- [x] libunistring (<https://github.com/rafalmiel/libunistring/tree/cykusz>)
- [x] libiconv (<https://github.com/rafalmiel/libiconv/tree/cykusz>)
- [x] libidn2 (<https://github.com/rafalmiel/libidn2/tree/cykusz>)
- [x] libffi (<https://github.com/rafalmiel/libffi/tree/cykusz>)
- [x] libexpat (<https://github.com/rafalmiel/libexpat/tree/cykusz>)

### Apps
- [x] binutils (<https://github.com/rafalmiel/binutils-gdb/tree/cykusz>)
- [x] gcc (<https://github.com/rafalmiel/gcc/tree/cykusz>)
- [x] llvm (<https://github.com/rafalmiel/llvm-project/tree/cykusz>)
- [x] zstd (<https://github.com/rafalmiel/zstd/tree/cykusz>)
- [x] coreutils (<https://github.com/rafalmiel/coreutils/tree/cykusz>)
- [x] nyancat (<https://github.com/rafalmiel/nyancat/tree/cykusz>)
- [x] bash (<https://github.com/rafalmiel/bash/tree/cykusz>)
- [x] nano (<https://github.com/rafalmiel/nano/tree/cykusz>)
- [x] less (<https://github.com/rafalmiel/less/tree/cykusz>)
- [x] doom (<https://github.com/rafalmiel/doomgeneric/tree/cykusz>)
- [x] netcat (<https://github.com/rafalmiel/netcat/tree/cykusz>)
- [x] wget (<https://github.com/rafalmiel/wget/tree/cykusz>)
- [x] python (<https://github.com/rafalmiel/cpython/tree/cykusz>)

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


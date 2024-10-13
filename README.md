[![CI](https://github.com/rafalmiel/cykusz-rs/actions/workflows/main.yml/badge.svg)](https://github.com/rafalmiel/cykusz-rs/actions/workflows/main.yml) [![CI](https://github.com/rafalmiel/cykusz-rs/actions/workflows/toolchain.yml/badge.svg)](https://github.com/rafalmiel/cykusz-rs/actions/workflows/toolchain_docker.yml)

# cykusz-rs

## Intro

cykusz-rs is a toy operating system written in Rust programming language.

https://github.com/rafalmiel/cykusz-rs/assets/3881998/afa514a1-f435-4eeb-8c80-200129e6900d.mov

## Kernel

### Core features
- [x] x86_64 monolithic kernel
- [x] 4-level paging
- [x] Preemptive per-cpu scheduler
- [x] ACPI (ioapic, lapic, acpica)
- [x] VM (elf loader, shared memory, COW)
- [x] Filesystem (ext2)
- [x] Page / Inode / Directory cache
- [x] TTY with ansi escape codes
- [x] Network Stack (e1000, ETH, ARP, IP, UDP, TCP, ICMP, DHCP, DNS)
- [x] IPC: Pipes / Unix Sockets / SHM

### Drivers
- [x] VESA framebuffer
- [x] PS/2: Keyboard / Mouse
- [x] Storage: IDE / AHCI
- [x] Networking: e1000
- [x] Sound: Intel HDA

## Userspace
- [x] libc (mlibc port)
- [x] Exec / fork
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
- [x] doom (with sound) (<https://github.com/rafalmiel/cykusz-rs/tree/master/userspace/doom>)
- [x] netcat (<https://github.com/rafalmiel/netcat/tree/cykusz>)
- [x] wget (<https://github.com/rafalmiel/wget/tree/cykusz>)
- [x] python (<https://github.com/rafalmiel/cpython/tree/cykusz>)

## Building OS

You will need following packages to compile and run the os:
* rust ([rustup](https://rustup.rs/))
* nasm
* qemu
* grub2
* parted
* docker (for userspace docker build)

### Building:
```bash
git clone https://github.com/rafalmiel/cykusz-rs.git
git submodule update --init --recursive

rustup override set nightly
rustup component add rust-src
make

./disk-scripts/create_disk.sh
```

## Building Userspace
It is recommended to use docker for building userspace for stable environment.

### Using docker
```bash
./sysroot/make_docker_image.sh
./sysroot/toolchain_docker.sh
```

### Using host
```bash
./sysroot/toolchain.sh
```

## Running
### qemu
```bash
make run
```

### bochs
```bash
make bochs
```

### VirtualBox
```bash
# Run only once to import the image into VirtualBox
./create_vbox_image.sh

make vbox
```


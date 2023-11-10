#!/bin/bash

set -x -e

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)
SRC_DIR=$CYKUSZ_DIR/sysroot/src
BINUTILS_SRC_DIR=$SRC_DIR/binutils-gdb
GCC_SRC_DIR=$SRC_DIR/gcc
MLIBC_SRC_DIR=$SRC_DIR/mlibc
NYANCAT_SRC_DIR=$SRC_DIR/nyancat
NCURSES_SRC_DIR=$SRC_DIR/ncurses
NANO_SRC_DIR=$SRC_DIR/nano
BASH_SRC_DIR=$SRC_DIR/bash
GMP_SRC_DIR=$SRC_DIR/gmp
MPFR_SRC_DIR=$SRC_DIR/mpfr
MPC_SRC_DIR=$SRC_DIR/mpc
DOOM_SRC_DIR=$SRC_DIR/doomgeneric
COREUTILS_SRC_DIR=$SRC_DIR/coreutils
TZDB_SRC_DIR=$SRC_DIR/tzdb
ZSTD_SRC_DIR=$SRC_DIR/zstd
LLVM_SRC_DIR=$SRC_DIR/llvm-project
LESS_SRC_DIR=$SRC_DIR/less
NETCAT_SRC_DIR=$SRC_DIR/netcat

BUILD_DIR=$CYKUSZ_DIR/sysroot/build
BINUTILS_BUILD_DIR=$BUILD_DIR/binutils-gdb
BINUTILS_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-binutils-gdb
GCC_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-gcc
NCURSES_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-ncurses
NANO_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-nano
BASH_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-bash
COREUTILS_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-coreutils
ZSTD_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-zstd
LLVM_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-llvm
LESS_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-less
NETCAT_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-netcat
GCC_BUILD_DIR=$BUILD_DIR/gcc
MLIBC_BUILD_DIR=$BUILD_DIR/mlibc
GMP_BUILD_DIR=$BUILD_DIR/gmp
MPFR_BUILD_DIR=$BUILD_DIR/mpfr
MPC_BUILD_DIR=$BUILD_DIR/mpc

SYSROOT=$CYKUSZ_DIR/sysroot/cykusz
CROSS=$CYKUSZ_DIR/sysroot/cross

TRIPLE=x86_64-cykusz

export PATH=$CYKUSZ_DIR/sysroot/bin:$CROSS/bin:$PATH

function _prepare_mlibc {
    if [ ! -d $MLIBC_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/mlibc.git $MLIBC_SRC_DIR
    fi
}

function _prepare_binutils {
    if [ ! -d $BINUTILS_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/binutils-gdb.git $BINUTILS_SRC_DIR
    fi
}

function _prepare_gcc {
    if [ ! -d $GCC_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/gcc.git $GCC_SRC_DIR

        pushd .

        cd $GCC_SRC_DIR
        ./contrib/download_prerequisites
        git apply patch-01.patch

        popd
    fi
}

function _prepare_nyancat {
    if [ ! -d $NYANCAT_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/nyancat.git $NYANCAT_SRC_DIR
    fi
}

function _prepare_doom {
    if [ ! -d $DOOM_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/doomgeneric.git $DOOM_SRC_DIR
    fi
}

function _prepare_ncurses {
    if [ ! -d $NCURSES_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/ncurses.git $NCURSES_SRC_DIR
    fi
}

function _prepare_bash {
    if [ ! -d $BASH_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/bash.git $BASH_SRC_DIR
    fi
}

function _prepare_coreutils {
    if [ ! -d $COREUTILS_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/coreutils.git $COREUTILS_SRC_DIR

        pushd .
        cd $COREUTILS_SRC_DIR
        ./bootstrap
        rm build-aux/config.sub
        mv config.sub.cykusz build-aux/config.sub
        popd
    fi
}

function _prepare_tzdb {
    if [ ! -d $TZDB_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/tzdb.git $TZDB_SRC_DIR
    fi
}

function _prepare_zstd {
    if [ ! -d $ZSTD_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/zstd.git $ZSTD_SRC_DIR
    fi
}

function _prepare_llvm {
    if [ ! -d $LLVM_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/llvm-project.git $LLVM_SRC_DIR
    fi
}

function _prepare_nano {
    if [ ! -d $NANO_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/nano.git $NANO_SRC_DIR

        pushd .
        cd $NANO_SRC_DIR
        ./autogen.sh
        rm config.sub
        mv config.sub.cykusz config.sub
        popd
    fi
}

function _prepare_less {
    if [ ! -d $LESS_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/less.git $LESS_SRC_DIR

        pushd .
        cd $LESS_SRC_DIR
        make -f Makefile.aut distfiles
        popd
    fi
}

function _prepare_netcat {
    if [ ! -d $NETCAT_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/netcat.git $NETCAT_SRC_DIR
    fi
}

LINUX_HEADERS_VERSION=6.6
LINUX_HEADERS_FOLDER=linux-$LINUX_HEADERS_VERSION
LINUX_HEADERS_TAR=$LINUX_HEADERS_FOLDER.tar.xz
LINUX_HEADERS_URL=https://cdn.kernel.org/pub/linux/kernel/v6.x/$LINUX_HEADERS_TAR
LINUX_HEADERS_SRC=$SRC_DIR/linux_headers

function _linux_headers {
    if [ ! -d $LINUX_HEADERS_SRC ]; then
        if [ ! -f $LINUX_HEADERS_TAR ]; then
            wget $LINUX_HEADERS_URL
            tar -xf $LINUX_HEADERS_TAR
        fi
        mkdir -p $LINUX_HEADERS_SRC
        pushd .
        cd $LINUX_HEADERS_FOLDER
        make headers_install INSTALL_HDR_PATH=$LINUX_HEADERS_SRC
        popd
    fi
}

function _sysroot {
    _prepare_mlibc

    mkdir -p $SYSROOT/usr/include
    cp -r $LINUX_HEADERS_SRC/include/asm $SYSROOT/usr/include/
    cp -r $LINUX_HEADERS_SRC/include/asm-generic $SYSROOT/usr/include/
    cp -r $LINUX_HEADERS_SRC/include/linux $SYSROOT/usr/include/

    mkdir -p $BUILD_DIR

    rm -rf $MLIBC_BUILD_DIR
    meson setup --cross-file $SPATH/cross-file.ini --prefix /usr -Dlinux_kernel_headers=$SYSROOT/usr/include -Dheaders_only=true $MLIBC_BUILD_DIR $MLIBC_SRC_DIR
    meson install -C $MLIBC_BUILD_DIR --destdir=$SYSROOT

    mkdir -p $SYSROOT/etc
    cp $SPATH/resolv.conf $SYSROOT/etc/
}

function _binutils {
    _prepare_binutils

    mkdir -p $BINUTILS_BUILD_DIR

    pushd .

    cd $BINUTILS_BUILD_DIR
    $BINUTILS_SRC_DIR/configure --target=$TRIPLE --prefix="$CROSS" --with-sysroot=$SYSROOT --disable-werror --disable-gdb --enable-shared

    popd

    make -C $BINUTILS_BUILD_DIR -j4
    make -C $BINUTILS_BUILD_DIR install

}

function _gcc {
    _prepare_gcc

    mkdir -p $GCC_BUILD_DIR

    pushd .

    cd $GCC_BUILD_DIR
    $GCC_SRC_DIR/configure --target=$TRIPLE --prefix="$CROSS" --with-sysroot=$SYSROOT --enable-languages=c,c++ --enable-threads=posix --enable-shared

    popd

    make -C $GCC_BUILD_DIR -j4 all-gcc
    make -C $GCC_BUILD_DIR install-gcc
}

function _mlibc {
    _prepare_mlibc

    mkdir -p $BUILD_DIR

    rm -rf $MLIBC_BUILD_DIR
    meson setup --cross-file $SPATH/cross-file.ini --prefix /usr -Ddefault_library=both -Dlinux_kernel_headers=$SYSROOT/usr/include -Dheaders_only=false $MLIBC_BUILD_DIR $MLIBC_SRC_DIR

    ninja -C $MLIBC_BUILD_DIR
    meson install -C $MLIBC_BUILD_DIR --destdir=$SYSROOT
}

function _dummy_libc {
    mkdir -p $SYSROOT/usr/lib
    $TRIPLE-gcc -nostdlib -nostartfiles -shared -x c /dev/null -o $SYSROOT/usr/lib/libc.so
}

function _libgcc {
    make -C $GCC_BUILD_DIR -j4 all-target-libgcc
    make -C $GCC_BUILD_DIR install-target-libgcc
}

function _libstd {
    make -C $GCC_BUILD_DIR -j4 all-target-libstdc++-v3
    make -C $GCC_BUILD_DIR install-target-libstdc++-v3
}

function _cykusz_tzdb {
    _prepare_tzdb

    make -C $TZDB_SRC_DIR
    make -C $TZDB_SRC_DIR DESTDIR=$SYSROOT install

    pushd .
    cd $SYSROOT/etc
    ln -sf ../usr/share/zoneinfo/Europe/London localtime

    cd $TZDB_SRC_DIR
    git clean -xfd
    popd
}

function _cykusz_binutils {
    _prepare_binutils

    mkdir -p $BINUTILS_CYKUSZ_BUILD_DIR

    pushd .

    cd $BINUTILS_CYKUSZ_BUILD_DIR

    $BINUTILS_SRC_DIR/configure --disable-gdb --disable-gdbserver --host=$TRIPLE --with-build-sysroot=$SYSROOT --disable-werror --enable-shared --prefix=/usr

    popd

    make -C $BINUTILS_CYKUSZ_BUILD_DIR -j4
    make -C $BINUTILS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _cykusz_gcc {
    _prepare_gcc

    mkdir -p $GCC_CYKUSZ_BUILD_DIR

    pushd .

    cd $GCC_CYKUSZ_BUILD_DIR
    $GCC_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --with-build-sysroot=$SYSROOT --enable-languages=c,c++ --enable-threads=posix --disable-multilib --enable-shared --prefix=/usr

    popd

    make -C $GCC_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4 all-gcc
    make -C $GCC_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install-gcc
}

function _cykusz_gcc_debug {
    _prepare_gcc

    mkdir -p $GCC_CYKUSZ_BUILD_DIR

    pushd .

    cd $GCC_CYKUSZ_BUILD_DIR
    $GCC_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --with-build-sysroot=$SYSROOT --enable-languages=c,c++ --enable-threads=posix --disable-multilib --enable-shared --prefix=/usr
    CXXFLAGS="-O0" CFLAGS="-O0" $GCC_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --with-build-sysroot=$SYSROOT --enable-languages=c,c++ --enable-threads=posix --disable-multilib --enable-shared --prefix=/usr

    popd

    make -C $GCC_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4 all-gcc
    make -C $GCC_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install-gcc
}

function _cykusz_libgcc {
    make -C $GCC_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4 all-target-libgcc
    make -C $GCC_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install-target-libgcc
}

function _cykusz_libstd {
    make -C $GCC_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4 all-target-libstdc++-v3
    make -C $GCC_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install-target-libstdc++-v3
}

function _cykusz_nyancat {
    _prepare_nyancat

    pushd .

    cd $NYANCAT_SRC_DIR/src
    make clean
    CC=$TRIPLE-gcc make
    cp nyancat $BUILD_DIR
    make clean

    popd
}

function _cykusz_doom {
    _prepare_doom

    pushd .

    cd $DOOM_SRC_DIR/doomgeneric
    CYKUSZ_ROOT=$SYSROOT make -f Makefile.cykusz
    cp fbdoom $BUILD_DIR
    cp ../DOOM1.WAD $BUILD_DIR
    make clean

    popd
}

function _cykusz_ncurses {
    _prepare_ncurses

    mkdir -p $NCURSES_CYKUSZ_BUILD_DIR

    pushd .

    cd $NCURSES_CYKUSZ_BUILD_DIR
    $NCURSES_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --prefix=/usr --without-tests --without-ada --with-shared --disable-stripping --with-debug --enable-widec

    popd

    make -C $NCURSES_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4
    make -C $NCURSES_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _cykusz_nano {
    _prepare_nano

    mkdir -p $NANO_CYKUSZ_BUILD_DIR

    pushd .

    cd $NANO_CYKUSZ_BUILD_DIR
    CFLAGS="-O0 -g" $NANO_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --prefix=/usr --disable-nanorc

    popd

    make -C $NANO_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT LIBS="-lncursesw" -j4
    make -C $NANO_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _cykusz_less {
    _prepare_less

    mkdir -p $LESS_CYKUSZ_BUILD_DIR

    pushd .

    cd $LESS_CYKUSZ_BUILD_DIR
    $LESS_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --prefix=/usr

    popd

    make -C $LESS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4
    make -C $LESS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _cykusz_netcat {
    _prepare_netcat

    mkdir -p $NETCAT_CYKUSZ_BUILD_DIR

    pushd .

    cd $NETCAT_CYKUSZ_BUILD_DIR
    $NETCAT_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --prefix=/usr

    popd

    make -C $NETCAT_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4
    make -C $NETCAT_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install

    pushd .
    cd $SYSROOT/usr/bin
    ln -sf $TRIPLE-netcat netcat
    popd
}

function _cykusz_coreutils {
    _prepare_coreutils

    mkdir -p $COREUTILS_CYKUSZ_BUILD_DIR

    pushd .

    cd $COREUTILS_CYKUSZ_BUILD_DIR

    CFLAGS="-DSLOW_BUT_NO_HACKS -Wno-error" $COREUTILS_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --prefix=/usr

    popd

    make -C $COREUTILS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4
    make -C $COREUTILS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _cykusz_bash {
    _prepare_bash

    mkdir -p $BASH_CYKUSZ_BUILD_DIR

    pushd .

    cd $BASH_CYKUSZ_BUILD_DIR
    $BASH_SRC_DIR/configure --host=$TRIPLE --prefix=/usr --without-bash-malloc --disable-nls

    popd

    make -C $BASH_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT
    make -C $BASH_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install

    ln -sf /usr/bin/bash $SYSROOT/usr/bin/sh
}

function _cykusz_llvm {
    _prepare_llvm

    mkdir -p $LLVM_CYKUSZ_BUILD_DIR

    pushd .

    cd $LLVM_CYKUSZ_BUILD_DIR

    export CYKUSZ_SYSROOT_DIR=$SYSROOT
    export CYKUSZ_ROOT_DIR=$SPATH
    cmake -DCMAKE_TOOLCHAIN_FILE=$SPATH/CMakeToolchain-x86_64-cykusz.txt -DLLVM_ENABLE_PROJECTS="clang;clang-tools-extra;lld" -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release -DLLVM_LINK_LLVM_DYLIB=ON -DLLVM_ENABLE_RTTI=ON -DLLVM_TARGETS_TO_BUILD=X86 -DLLVM_TARGET_ARCH=x86_64 -DLLVM_DEFAULT_TARGET_TRIPLE=$TRIPLE -DLLVM_HOST_TRIPLE=$TRIPLE -Wno-dev $LLVM_SRC_DIR/llvm

    VERBOSE=1 make -j12 DESTDIR=$SYSROOT
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_zstd {
    _prepare_zstd

    mkdir -p $ZSTD_CYKUSZ_BUILD_DIR

    pushd .

    cd $ZSTD_CYKUSZ_BUILD_DIR

    cmake -DCMAKE_TOOLCHAIN_FILE=$SPATH/CMakeToolchain-x86_64-cykusz.txt -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release $ZSTD_SRC_DIR/build/cmake
    make -j8
    DESTDIR=$SYSROOT make install

    popd
}

function _prepare {
    _prepare_mlibc
    _prepare_binutils
    _prepare_gcc
    _prepare_nyancat
    _prepare_ncurses
    _prepare_bash
    _prepare_nano
    _prepare_less
    _prepare_zstd
    _prepare_doom
}

function _cross {
    _sysroot
    _binutils
    _gcc
    _mlibc
    _libgcc
    _libstd
}

function _cykusz {
    _cykusz_binutils
    _cykusz_gcc
    _cykusz_libgcc
    _cykusz_libstd
    _cykusz_nyancat
    _cykusz_ncurses
    _cykusz_bash
    _cykusz_nano
    _cykusz_less
    _cykusz_zstd
    _cykusz_doom
}

function _build {
    _cross
    _cykusz
}

function _all {
    _prepare
    _build
}

function _clean {
    rm -rf $BUILD_DIR
    rm -rf $CROSS
    rm -rf $SYSROOT
}

function _check_build {
    if [ ! -f $CROSS/bin/$TRIPLE-gcc ]; then
        _all
    fi
}

if [ -z "$1" ]; then
    echo "Usage: $0 (clean/prepare/binutils/gcc/mlibc/cykusz_nyancat/cykusz_ncurses/cykusz_bash/cykusz_nano/check_build/build/all)"
else
    cd $SPATH
    _$1
fi

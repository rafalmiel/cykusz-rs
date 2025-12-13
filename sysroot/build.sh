#!/bin/bash

set -x -e

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)
SRC_DIR=$CYKUSZ_DIR/sysroot/src
BINUTILS_SRC_DIR=$SRC_DIR/binutils-gdb
GCC_SRC_DIR=$SRC_DIR/gcc
RUST_SRC_DIR=$SRC_DIR/rust
AUTOMAKE_SRC_DIR=$SRC_DIR/automake
AUTOCONF_SRC_DIR=$SRC_DIR/autoconf
AUTOCONF_269_SRC_DIR=$SRC_DIR/autoconf_269
LIBTOOL_SRC_DIR=$SRC_DIR/libtool
PKGCONF_SRC_DIR=$SRC_DIR/pkgconf
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
LIBRESSL_SRC_DIR=$SRC_DIR/libressl-portable
LLVM_SRC_DIR=$SRC_DIR/llvm-project
LESS_SRC_DIR=$SRC_DIR/less
NETCAT_SRC_DIR=$SRC_DIR/netcat
ZLIB_SRC_DIR=$SRC_DIR/zlib
PYTHON_SRC_DIR=$SRC_DIR/cpython
READLINE_SRC_DIR=$SRC_DIR/readline
WGET_SRC_DIR=$SRC_DIR/wget
WGET2_SRC_DIR=$SRC_DIR/wget2
LIBPSL_SRC_DIR=$SRC_DIR/libpsl
PCRE2_SRC_DIR=$SRC_DIR/pcre2
LIBUNISTRING_SRC_DIR=$SRC_DIR/libunistring
LIBICONV_SRC_DIR=$SRC_DIR/libiconv
LIBIDN2_SRC_DIR=$SRC_DIR/libidn2
LIBFFI_SRC_DIR=$SRC_DIR/libffi
LIBEXPAT_SRC_DIR=$SRC_DIR/libexpat

BUILD_DIR=$CYKUSZ_DIR/sysroot/build
BINUTILS_BUILD_DIR=$BUILD_DIR/binutils-gdb
BINUTILS_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-binutils-gdb
GCC_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-gcc
AUTOMAKE_BUILD_DIR=$BUILD_DIR/automake
AUTOCONF_BUILD_DIR=$BUILD_DIR/autoconf
AUTOCONF_269_BUILD_DIR=$BUILD_DIR/autoconf_269
LIBTOOL_BUILD_DIR=$BUILD_DIR/libtool
PKGCONF_BUILD_DIR=$BUILD_DIR/pkgconf
NCURSES_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-ncurses
NANO_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-nano
BASH_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-bash
COREUTILS_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-coreutils
ZSTD_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-zstd
LIBRESSL_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-libressl
LLVM_BUILD_DIR=$BUILD_DIR/llvm
LLVM_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-llvm
LESS_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-less
NETCAT_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-netcat
ZLIB_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-zlib
PYTHON_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-python
READLINE_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-readline
WGET_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-wget
WGET2_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-wget2
LIBPSL_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-libpsl
PCRE2_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-pcre2
LIBUNISTRING_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-libunistring
LIBICONV_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-libiconv
LIBIDN2_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-libidn2
LIBFFI_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-libffi
LIBEXPAT_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-libexpat
GCC_BUILD_DIR=$BUILD_DIR/gcc
RUST_BUILD_DIR=$BUILD_DIR/rust
MLIBC_BUILD_DIR=$BUILD_DIR/mlibc

SYSROOT=$CYKUSZ_DIR/sysroot/cykusz
CROSS=$CYKUSZ_DIR/sysroot/cross/usr
CROSSOPT=$CYKUSZ_DIR/sysroot/cross/opt

TRIPLE=x86_64-cykusz

export PATH=$CYKUSZ_DIR/sysroot/bin:$CROSS/bin:$PATH
export ACLOCAL_PATH=$CROSS/share/aclocal

export PKG_CONFIG=$CROSS/bin/pkgconf
export PKG_CONFIG_SYSROOT_DIR=$SYSROOT
export PKG_CONFIG_PATH=$SYSROOT/usr/lib/pkgconfig:$SYSROOT/usr/share/pkgconfig
export PKG_CONFIG_LIBDIR=$SYSROOT/usr/lib/pkgconfig:$SYSROOT/usr/share/pkgconfig

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

        cd libstdc++-v3/
        $CROSSOPT/autoconf-2.69/bin/autoreconf -i
        cd ../lto-plugin
        $CROSSOPT/autoconf-2.69/bin/autoreconf -i
        cd ../libbacktrace
        $CROSSOPT/autoconf-2.69/bin/autoreconf -i

        popd
    fi
}

function _prepare_rust {
    if [ ! -d $RUST_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone -b cykusz https://github.com/rafalmiel/rust.git $RUST_SRC_DIR

        pushd .

        cd $RUST_SRC_DIR
        cat > bootstrap.toml <<EOL
change-id = 146435

[llvm]
download-ci-llvm = false
targets = "X86"

[build]
target = ["x86_64-unknown-linux-gnu", "x86_64-unknown-cykusz"]
build-dir = "$RUST_BUILD_DIR"
docs = false

[install]
prefix = ""
sysconfdir = "etc"

[rust]
debug = false
codegen-tests = false
deny-warnings = false # work around rust-num-cpus warning
channel = "nightly"

[target.x86_64-unknown-linux-gnu]
llvm-config = "$CROSS/bin/llvm-config"

[target.x86_64-unknown-cykusz]
llvm-config = "$CROSS/bin/llvm-config"
EOL

        popd
    fi
}

function _prepare_automake {
    if [ ! -d $AUTOMAKE_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone -b cykusz https://github.com/rafalmiel/automake.git $AUTOMAKE_SRC_DIR

        pushd .

        cd $AUTOMAKE_SRC_DIR
        git submodule update --init
        ./bootstrap

        popd
    fi
}

function _prepare_autoconf {
    if [ ! -d $AUTOCONF_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone -b cykusz https://github.com/rafalmiel/autoconf.git $AUTOCONF_SRC_DIR

        pushd .

        cd $AUTOCONF_SRC_DIR
        ./bootstrap

        ARCHIVE_VER="2024.10.16"
        mkdir -p autoconf-archive && cd autoconf-archive
        wget https://ftp.gnu.org/gnu/autoconf-archive/autoconf-archive-${ARCHIVE_VER}.tar.xz
        tar xf autoconf-archive-${ARCHIVE_VER}.tar.xz
        mkdir -p $CROSS/share/aclocal
        cp -r autoconf-archive-${ARCHIVE_VER}/m4/* $CROSS/share/aclocal/

        popd
    fi
}


function _prepare_autoconf_269 {
    if [ ! -d $AUTOCONF_269_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone -b cykusz https://github.com/rafalmiel/autoconf-2.69.git $AUTOCONF_269_SRC_DIR
    fi
}

function _prepare_libtool {
    if [ ! -d $LIBTOOL_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone -b cykusz https://github.com/rafalmiel/libtool.git $LIBTOOL_SRC_DIR

        pushd .

        cd $LIBTOOL_SRC_DIR
        git submodule update --init
        ./bootstrap

        popd
    fi
}

function _prepare_pkgconf {
    if [ ! -d $PKGCONF_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone -b cykusz https://github.com/rafalmiel/pkgconf.git $PKGCONF_SRC_DIR

        pushd .

        cd $PKGCONF_SRC_DIR
        git submodule update --init
        ./autogen.sh

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
        ln -sf ../gnulib/build-aux/config.sub build-aux/config.sub
        popd
    fi
}

function _prepare_wget {
    if [ ! -d $WGET_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/wget.git $WGET_SRC_DIR

        pushd .
        cd $WGET_SRC_DIR
        ./bootstrap
        popd
    fi
}

function _prepare_wget2 {
    if [ ! -d $WGET2_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/wget2.git $WGET2_SRC_DIR

        pushd .
        cd $WGET2_SRC_DIR
        ./bootstrap
        popd
    fi
}

function _prepare_libpsl {
    if [ ! -d $LIBPSL_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/libpsl.git $LIBPSL_SRC_DIR

        pushd .
        cd $LIBPSL_SRC_DIR
        ./autogen.sh
        rm build-aux/config.sub
        mv config.sub.cykusz build-aux/config.sub
        popd
    fi
}

function _prepare_libunistring {
    if [ ! -d $LIBUNISTRING_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/libunistring.git $LIBUNISTRING_SRC_DIR

        pushd .
        cd $LIBUNISTRING_SRC_DIR
        ./autopull.sh
        ./autogen.sh
        cp config.sub.cykusz gnulib/build-aux/config.sub
        cp config.sub.cykusz build-aux/config.sub
        popd
    fi
}

function _prepare_libffi {
    if [ ! -d $LIBFFI_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/libffi.git $LIBFFI_SRC_DIR

        pushd .
        cd $LIBFFI_SRC_DIR
        ./autogen.sh
        popd
    fi
}

function _prepare_libexpat {
    if [ ! -d $LIBEXPAT_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/libexpat.git $LIBEXPAT_SRC_DIR

        pushd .
        cd $LIBEXPAT_SRC_DIR/expat
        ./buildconf.sh
        cp config.sub.cykusz conftools/config.sub
        popd
    fi
}

function _prepare_libiconv {
    if [ ! -d $LIBICONV_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/libiconv.git $LIBICONV_SRC_DIR

        pushd .
        cd $LIBICONV_SRC_DIR
        ./gitsub.sh pull
        ./autogen.sh
        cp config.sub.cykusz gnulib/build-aux/config.sub
        cp config.sub.cykusz libcharset/build-aux/config.sub
        cp config.sub.cykusz build-aux/config.sub
        popd
    fi
}

function _prepare_libidn2 {
    if [ ! -d $LIBIDN2_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone -b cykusz https://github.com/rafalmiel/libidn2.git $LIBIDN2_SRC_DIR

        pushd .
        cd $LIBIDN2_SRC_DIR
        ./bootstrap
        cp config.sub.cykusz gnulib/build-aux/config.sub
        cp config.sub.cykusz build-aux/config.sub
        popd
    fi
}

function _prepare_pcre2 {
    if [ ! -d $PCRE2_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz --recursive https://github.com/rafalmiel/pcre2.git $PCRE2_SRC_DIR

        pushd .
        cd $PCRE2_SRC_DIR
        ./autogen.sh
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

function _prepare_zlib {
    if [ ! -d $ZLIB_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/zlib.git $ZLIB_SRC_DIR
    fi
}

function _prepare_libressl {
    if [ ! -d $LIBRESSL_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/libressl-portable.git $LIBRESSL_SRC_DIR

        pushd .
        cd $LIBRESSL_SRC_DIR
        ./autogen.sh
        popd
    fi
}

function _prepare_python {
    if [ ! -d $PYTHON_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/cpython.git $PYTHON_SRC_DIR

        pushd .
        cd $PYTHON_SRC_DIR
        autoreconf -f -i
        popd
    fi
}

function _prepare_readline {
    if [ ! -d $READLINE_SRC_DIR ]; then
        mkdir -p $SRC_DIR
        git clone --depth 1 -b cykusz https://github.com/rafalmiel/readline.git $READLINE_SRC_DIR
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

LINUX_HEADERS_VERSION=6.18
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
    meson setup --cross-file $SPATH/cfg/cross-file.ini --prefix /usr -Dlinux_kernel_headers=$SYSROOT/usr/include -Dheaders_only=true $MLIBC_BUILD_DIR $MLIBC_SRC_DIR
    meson install -C $MLIBC_BUILD_DIR --destdir=$SYSROOT

    mkdir -p $SYSROOT/etc
    cp $SPATH/cfg/resolv.conf $SYSROOT/etc/
}

function _sysroot_rust_bindings {
    _prepare_mlibc

    SYSROOT_BIND=${SYSROOT}_mlibc
    rm -rf $SYSROOT_BIND

    mkdir -p $BUILD_DIR

    rm -rf $MLIBC_BUILD_DIR
    meson setup --cross-file $SPATH/cfg/cross-file.ini --prefix /usr -Dlinux_kernel_headers=${LINUX_HEADERS_SRC}/include -Dheaders_only=true $MLIBC_BUILD_DIR $MLIBC_SRC_DIR
    meson install -C $MLIBC_BUILD_DIR --destdir=${SYSROOT_BIND}
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

function _rust {
    _prepare_rust

    pushd .

    cd $RUST_SRC_DIR
    CARGO_HOME=$BUILD_DIR/cargo_home
    mkdir -p $CARGO_HOME

    cp $SPATH/cfg/rust/host-config.toml $CARGO_HOME/config.toml

    INSTALL_PATH="$SPATH/cross"

    rustup component add rust-src

    CARGO_HOME=$CARGO_HOME ./x.py build --stage 2 -j12
    CARGO_HOME=$CARGO_HOME DESTDIR="$INSTALL_PATH/usr/local" ./x.py install
    CARGO_HOME=$CARGO_HOME DESTDIR="$INSTALL_PATH/usr/local" ./x.py dist

    cd $BUILD_DIR/rust/dist
    tar -xf rust-src-nightly.tar.xz
    cd rust-src-nightly
    ./install.sh --destdir="$INSTALL_PATH"

    cd "$INSTALL_PATH/usr/local/bin"
    ln -sf "$(which cargo)" .

    popd
}

function _llvm {
    _prepare_llvm

    mkdir -p $LLVM_BUILD_DIR

    pushd .

    cd $LLVM_BUILD_DIR

    cmake -DLLVM_ENABLE_PROJECTS="clang;clang-tools-extra;lld" -DDEFAULT_SYSROOT=$SYSROOT -DCMAKE_INSTALL_PREFIX="$CROSS" -DCMAKE_BUILD_TYPE=RelWithDebInfo -DLLVM_LINK_LLVM_DYLIB=ON -DLLVM_ENABLE_RTTI=ON -DLLVM_TARGETS_TO_BUILD=X86 -DLLVM_TARGET_ARCH=x86_64 -DLLVM_DEFAULT_TARGET_TRIPLE=$TRIPLE -Wno-dev $LLVM_SRC_DIR/llvm

    VERBOSE=1 make -j12
    make install

    popd
}

function _automake {
    _prepare_automake

    mkdir -p $AUTOMAKE_BUILD_DIR

    pushd .

    cd $AUTOMAKE_BUILD_DIR
    $AUTOMAKE_SRC_DIR/configure --prefix=$CROSS

    make -j4
    make install

    popd
}

function _autoconf {
    _prepare_autoconf

    mkdir -p $AUTOCONF_BUILD_DIR

    pushd .

    cd $AUTOCONF_BUILD_DIR
    $AUTOCONF_SRC_DIR/configure --prefix=$CROSS

    make -j4
    make install

    popd
}

function _autoconf_269 {
    _prepare_autoconf_269

    mkdir -p $AUTOCONF_269_BUILD_DIR

    pushd .

    cd $AUTOCONF_269_BUILD_DIR
    $AUTOCONF_269_SRC_DIR/configure --prefix=$CROSSOPT/autoconf-2.69

    make install

    popd
}

function _libtool {
    _prepare_libtool

    mkdir -p $LIBTOOL_BUILD_DIR

    pushd .

    cd $LIBTOOL_BUILD_DIR
    $LIBTOOL_SRC_DIR/configure --prefix=$CROSS

    make -j4
    make install

    popd
}

function _pkgconf {
    _prepare_pkgconf

    mkdir -p $PKGCONF_BUILD_DIR

    pushd .

    cd $PKGCONF_BUILD_DIR
    $PKGCONF_SRC_DIR/configure --prefix=$CROSS

    make -j4
    make install

    popd
}

function _mlibc {
    _prepare_mlibc

    mkdir -p $BUILD_DIR

    rm -rf $MLIBC_BUILD_DIR
    meson setup --cross-file $SPATH/cfg/cross-file.ini --prefix /usr -Ddefault_library=both -Dlinux_kernel_headers=$SYSROOT/usr/include -Dheaders_only=false $MLIBC_BUILD_DIR $MLIBC_SRC_DIR

    ninja -C $MLIBC_BUILD_DIR
    meson install -C $MLIBC_BUILD_DIR --destdir=$SYSROOT
}

function _dummy_libc {
    mkdir -p $SYSROOT/usr/lib
    $TRIPLE-gcc -nostdlib -nostartfiles -shared -x c /dev/null -o $SYSROOT/usr/lib/libc.so
    $TRIPLE-gcc -nostdlib -nostartfiles -shared -x c /dev/null -o $SYSROOT/usr/lib/crti.o
    $TRIPLE-gcc -nostdlib -nostartfiles -shared -x c /dev/null -o $SYSROOT/usr/lib/crtn.o
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

    $BINUTILS_SRC_DIR/configure --disable-gdb --disable-gdbserver --host=$TRIPLE --target=$TRIPLE --with-build-sysroot=$SYSROOT --disable-werror --enable-shared --prefix=/usr

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
    $GCC_SRC_DIR/configure --host=$TRIPLE --with-build-sysroot=$SYSROOT --enable-languages=c,c++ --enable-threads=posix --disable-multilib --enable-shared --prefix=/usr
    CXXFLAGS="-O0" CFLAGS="-O0" $GCC_SRC_DIR/configure --host=$TRIPLE --with-build-sysroot=$SYSROOT --enable-languages=c,c++ --enable-threads=posix --disable-multilib --enable-shared --prefix=/usr

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
    cp ../doom1.wad $BUILD_DIR
    make clean

    popd
}

function _cykusz_ncurses {
    _prepare_ncurses

    mkdir -p $NCURSES_CYKUSZ_BUILD_DIR

    pushd .

    cd $NCURSES_CYKUSZ_BUILD_DIR
    $NCURSES_SRC_DIR/configure --host=$TRIPLE --prefix=/usr --without-tests --without-ada --with-shared --disable-stripping --with-debug --enable-widec --enable-pc-files --with-pkg-config-libdir=/usr/lib/pkgconfig --with-termlib

    popd

    make -C $NCURSES_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4
    make -C $NCURSES_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install

    for lib in ncurses form panel menu tinfo; do
        rm -vf ${SYSROOT}/usr/lib/lib${lib}.so
        echo "INPUT(-l${lib}w)" > ${SYSROOT}/usr/lib/lib${lib}.so
        ln -svf ${lib}w.pc "${SYSROOT}/usr/lib/pkgconfig/${lib}.pc"
    done

    rm -vf ${SYSROOT}/usr/lib/libcursesw.so
    echo "INPUT(-lncursesw)" > ${SYSROOT}/usr/lib/libcursesw.so
    ln -svf libncurses.so "${SYSROOT}/usr/lib/libcurses.so"
}

function _cykusz_nano {
    _prepare_nano

    mkdir -p $NANO_CYKUSZ_BUILD_DIR

    pushd .

    cd $NANO_CYKUSZ_BUILD_DIR
    $NANO_SRC_DIR/configure --host=$TRIPLE --prefix=/usr --enable-nanorc --enable-color

    popd

    make -C $NANO_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4
    make -C $NANO_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _cykusz_less {
    _prepare_less

    mkdir -p $LESS_CYKUSZ_BUILD_DIR

    pushd .

    cd $LESS_CYKUSZ_BUILD_DIR
    $LESS_SRC_DIR/configure --host=$TRIPLE --prefix=/usr CFLAGS="-O2 -g -std=gnu89"

    popd

    make -C $LESS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4
    make -C $LESS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _cykusz_netcat {
    _prepare_netcat

    mkdir -p $NETCAT_CYKUSZ_BUILD_DIR

    pushd .

    cd $NETCAT_CYKUSZ_BUILD_DIR
    $NETCAT_SRC_DIR/configure --host=$TRIPLE --prefix=/usr

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

    CFLAGS="-Wno-error" $COREUTILS_SRC_DIR/configure --host=$TRIPLE --prefix=/usr

    popd

    make -C $COREUTILS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT -j4
    make -C $COREUTILS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _cykusz_wget {
    _prepare_wget

    mkdir -p $WGET_CYKUSZ_BUILD_DIR

    pushd .

    cd $WGET_CYKUSZ_BUILD_DIR

    $WGET_SRC_DIR/configure --host=$TRIPLE --prefix=/usr --sysconfdir=/etc --disable-nls --with-ssl=openssl --with-openssl

    make DESTDIR=$SYSROOT -j4
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_wget2 {
    _prepare_wget2

    mkdir -p $WGET2_CYKUSZ_BUILD_DIR

    pushd .

    cd $WGET2_CYKUSZ_BUILD_DIR

    $WGET2_SRC_DIR/configure --host=$TRIPLE --exec-prefix=/usr --prefix=/usr --sysconfdir=/etc --disable-nls --with-ssl=openssl --with-openssl

    make DESTDIR=$SYSROOT -j4 VERBOSE=1
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_libpsl {
    _prepare_libpsl

    mkdir -p $LIBPSL_CYKUSZ_BUILD_DIR

    pushd .

    cd $LIBPSL_CYKUSZ_BUILD_DIR

    $LIBPSL_SRC_DIR/configure --host=$TRIPLE --with-sysroot=$SYSROOT --prefix=/usr --disable-static --disable-asan --disable-cfi --disable-ubsan --disable-man --disable-runtime

    make DESTDIR=$SYSROOT -j4
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_libunistring {
    _prepare_libunistring

    mkdir -p $LIBUNISTRING_CYKUSZ_BUILD_DIR

    pushd .

    cd $LIBUNISTRING_CYKUSZ_BUILD_DIR

    $LIBUNISTRING_SRC_DIR/configure --host=$TRIPLE  --prefix=/usr --with-sysroot=$SYSROOT --disable-static --docdir=/usr/share/doc/libunisttring-1.3

    make DESTDIR=$SYSROOT -j4
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_libffi {
    _prepare_libffi

    mkdir -p $LIBFFI_CYKUSZ_BUILD_DIR

    pushd .

    cd $LIBFFI_CYKUSZ_BUILD_DIR

    CFLAGS="-fPIC" $LIBFFI_SRC_DIR/configure --host=$TRIPLE  --prefix=/usr --with-sysroot=$SYSROOT --disable-static

    make DESTDIR=$SYSROOT -j4
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_libexpat {
    _prepare_libexpat

    mkdir -p $LIBEXPAT_CYKUSZ_BUILD_DIR

    pushd .

    cd $LIBEXPAT_CYKUSZ_BUILD_DIR

    $LIBEXPAT_SRC_DIR/expat/configure --host=$TRIPLE --prefix=/usr --with-sysroot=$SYSROOT --without-xmlwf

    make DESTDIR=$SYSROOT -j4
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_libiconv {
    _prepare_libiconv

    mkdir -p $LIBICONV_CYKUSZ_BUILD_DIR

    pushd .

    cd $LIBICONV_CYKUSZ_BUILD_DIR

    $LIBICONV_SRC_DIR/configure --host=$TRIPLE  --prefix=/usr --with-sysroot=$SYSROOT --enable-shared --disable-nls --disable-static

    make DESTDIR=$SYSROOT -j4
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_libidn2 {
    _prepare_libidn2

    rm -rf $LIBIDN2_CYKUSZ_BUILD_DIR
    mkdir -p $LIBIDN2_CYKUSZ_BUILD_DIR

    pushd .

    cd $LIBIDN2_CYKUSZ_BUILD_DIR

    $LIBIDN2_SRC_DIR/configure --disable-doc --disable-nls
    cp ./lib/idn2.h $LIBIDN2_SRC_DIR/lib/

    cd lib

    if [ ! -f $LIBIDN2_SRC_DIR/lib/gendata ]; then
        make gendata
        cp ./gendata $LIBIDN2_SRC_DIR/lib/gendata
    else
        cp $LIBIDN2_SRC_DIR/lib/gendata ./gendata
    fi
    if [ ! -f $LIBIDN2_SRC_DIR/lib/gentr46map ]; then
        make gentr46map
        cp ./gentr46map $LIBIDN2_SRC_DIR/lib/gentr46map
    else
        cp $LIBIDN2_SRC_DIR/lib/gentr46map ./gentr46map
    fi
    cd ../

    $LIBIDN2_SRC_DIR/configure --host=$TRIPLE  --prefix=/usr --with-sysroot=$SYSROOT --disable-nls --disable-static --disable-doc

    cp ./lib/idn2.h $LIBIDN2_SRC_DIR/lib/

    VERBOSE=1 make DESTDIR=$SYSROOT -j4
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_pcre2 {
    _prepare_pcre2

    mkdir -p $PCRE2_CYKUSZ_BUILD_DIR

    pushd .

    cd $PCRE2_CYKUSZ_BUILD_DIR

    $PCRE2_SRC_DIR/configure --host=$TRIPLE  --prefix=/usr --with-sysroot=$SYSROOT --docdir=/usr/share/doc/pcre2-10.47 --enable-unicode --enable-jit --enable-pcre2-16 --enable-pcre2-32 --enable-pcre2grep-libz --enable-pcre2test-libreadline --disable-static

    make DESTDIR=$SYSROOT -j4
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_bash {
    _prepare_bash

    mkdir -p $BASH_CYKUSZ_BUILD_DIR

    pushd .

    cd $BASH_CYKUSZ_BUILD_DIR
    $BASH_SRC_DIR/configure --host=$TRIPLE --target=$TRIPLE --prefix=/usr --without-bash-malloc --disable-nls --with-installed-readline CFLAGS="-O2 -g -std=gnu89"

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
    cmake -DCMAKE_TOOLCHAIN_FILE=$SPATH/cfg/CMakeToolchain-x86_64-cykusz.txt -DLLVM_ENABLE_PROJECTS="clang;clang-tools-extra;lld" -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release -DLLVM_LINK_LLVM_DYLIB=ON -DLLVM_ENABLE_RTTI=ON -DLLVM_TARGETS_TO_BUILD=X86 -DLLVM_TARGET_ARCH=x86_64 -DLLVM_DEFAULT_TARGET_TRIPLE=$TRIPLE -DLLVM_HOST_TRIPLE=$TRIPLE -Wno-dev $LLVM_SRC_DIR/llvm

    VERBOSE=1 make -j12 DESTDIR=$SYSROOT
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_zstd {
    _prepare_zstd

    mkdir -p $ZSTD_CYKUSZ_BUILD_DIR

    pushd .

    cd $ZSTD_CYKUSZ_BUILD_DIR

    cmake -DCMAKE_TOOLCHAIN_FILE=$SPATH/cfg/CMakeToolchain-x86_64-cykusz.txt -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release $ZSTD_SRC_DIR/build/cmake
    make -j8
    DESTDIR=$SYSROOT make install

    popd
}

function _cykusz_zlib {
    _prepare_zlib

    mkdir -p $ZLIB_CYKUSZ_BUILD_DIR

    pushd .

    cd $ZLIB_CYKUSZ_BUILD_DIR

    export CYKUSZ_SYSROOT_DIR=$SYSROOT
    export CYKUSZ_ROOT_DIR=$SPATH
    cmake -DCMAKE_TOOLCHAIN_FILE=$SPATH/cfg/CMakeToolchain-x86_64-cykusz.txt  -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release $ZLIB_SRC_DIR

    VERBOSE=1 make -j12 DESTDIR=$SYSROOT
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_libressl {
    _prepare_libressl

    mkdir -p $LIBRESSL_CYKUSZ_BUILD_DIR

    pushd .

    cd $LIBRESSL_CYKUSZ_BUILD_DIR

    export CYKUSZ_SYSROOT_DIR=$SYSROOT
    export CYKUSZ_ROOT_DIR=$SPATH
    cmake -DCMAKE_TOOLCHAIN_FILE=$SPATH/cfg/CMakeToolchain-x86_64-cykusz.txt -DBUILD_SHARED_LIBS=ON -DLIBRESSL_APPS=ON -DENABLE_NC=OFF -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release $LIBRESSL_SRC_DIR

    VERBOSE=1 make -j12 DESTDIR=$SYSROOT
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_python {
    _prepare_python

    pushd .

    mkdir -p $PYTHON_CYKUSZ_BUILD_DIR

    cd $PYTHON_CYKUSZ_BUILD_DIR
    export CONFIG_SITE=$SPATH/cfg/python-config-site
    $PYTHON_SRC_DIR/configure --with-build-python=python3.12 --host=$TRIPLE --build=x86_64-linux-gnu --prefix=/usr --enable-shared --disable-ipv6 --without-static-libpython --without-ensurepip --with-readline=readline

    make -j6 DESTDIR=$SYSROOT
    make DESTDIR=$SYSROOT install

    ln -sf python3 $SYSROOT/usr/bin/python

    popd
}

function _cykusz_readline {
    _prepare_readline

    pushd .

    mkdir -p $READLINE_CYKUSZ_BUILD_DIR

    cd $READLINE_CYKUSZ_BUILD_DIR
    $READLINE_SRC_DIR/configure --host=$TRIPLE --prefix=/usr --disable-static --enable-multibyte --with-curses

    make -j6 SHLIB_LIBS="-lncurses -ltinfo" DESTDIR=$SYSROOT
    make DESTDIR=$SYSROOT install

    popd
}

function _cykusz_apps {
    $TRIPLE-gcc $SRC_DIR/cykusz_apps/test.c -o $BUILD_DIR/test
    $TRIPLE-gcc $SRC_DIR/cykusz_apps/open_sleep.c -o $BUILD_DIR/open_sleep
    $TRIPLE-gcc  $SRC_DIR/cykusz_apps/stack.c -o $BUILD_DIR/stack
    $TRIPLE-g++ $SRC_DIR/cykusz_apps/hello.cpp -o $BUILD_DIR/hello
    $TRIPLE-g++ $SRC_DIR/cykusz_apps/test.cpp -o $BUILD_DIR/testcpp
    $TRIPLE-gcc $SRC_DIR/cykusz_apps/ttytest.c -o $BUILD_DIR/ttytest
    $TRIPLE-gcc $SRC_DIR/cykusz_apps/fork.c -o $BUILD_DIR/fork
    $TRIPLE-gcc $SRC_DIR/cykusz_apps/forktest.c -o $BUILD_DIR/forktest
    $TRIPLE-gcc $SRC_DIR/cykusz_apps/poweroff.c -o $BUILD_DIR/poweroff
    $TRIPLE-gcc $SRC_DIR/cykusz_apps/stat.c -o $BUILD_DIR/stat
    _cykusz_nyancat
}

function _cargo_userspace {
    pushd .
    cd $CYKUSZ_DIR/userspace
    cargo build --release
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

#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))

cd $SPATH/..
export CYKUSZ_DIR=$(pwd)
cd $SPATH

set -x

SRC_DIR=$CYKUSZ_DIR/sysroot/src
BINUTILS_SRC_DIR=$SRC_DIR/binutils-gdb
GCC_SRC_DIR=$SRC_DIR/gcc
MLIBC_SRC_DIR=$SRC_DIR/mlibc

BUILD_DIR=$CYKUSZ_DIR/sysroot/build
BINUTILS_BUILD_DIR=$BUILD_DIR/binutils-gdb
GCC_BUILD_DIR=$BUILD_DIR/gcc
MLIBC_BUILD_DIR=$BUILD_DIR/mlibc

SYSROOT=$CYKUSZ_DIR/sysroot/cykusz
CROSS=$CYKUSZ_DIR/sysroot/cross

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
	fi
}

function _prepare {
	_prepare_mlibc
	_prepare_binutils
	_prepare_gcc
}

function _sysroot {
	mkdir -p $BUILD_DIR

	rm -rf $MLIBC_BUILD_DIR
	meson setup --cross=$SPATH/cross-file.ini -Dheaders_only=true -Dstatic=true $MLIBC_SRC_DIR $MLIBC_BUILD_DIR --prefix=$SYSROOT/usr
	
	pushd .

	cd $MLIBC_BUILD_DIR
	meson install

	popd
}

function _binutils {
	mkdir -p $BUILD_DIR

	pushd .

	mkdir -p $BINUTILS_BUILD_DIR
	cd $BINUTILS_BUILD_DIR
	$BINUTILS_SRC_DIR/configure --target=x86_64-cykusz --prefix="$CROSS" --with-sysroot=$SYSROOT --disable-werror --disable-gdb
	make -j4
	make install

	popd
}

function _gcc {
	mkdir -p $BUILD_DIR

	pushd .

	mkdir -p $GCC_BUILD_DIR
	cd $GCC_BUILD_DIR
	$GCC_SRC_DIR/configure --target=x86_64-cykusz --prefix="$CROSS" --with-sysroot=$SYSROOT --enable-languages=c,c++
	make -j4 all-gcc all-target-libgcc
	make install-gcc install-target-libgcc

	popd
}

function _mlibc {
	mkdir -p $BUILD_DIR
	
	OLDPATH=$PATH
	export PATH=$CROSS/bin:$PATH 

	rm -rf $MLIBC_BUILD_DIR
	meson setup --cross=$SPATH/cross-file.ini -Dheaders_only=false -Dstatic=true $MLIBC_SRC_DIR $MLIBC_BUILD_DIR --prefix=$SYSROOT/usr
	
	pushd .

	cd $MLIBC_BUILD_DIR
	meson compile
	meson install

	popd

	export PATH=$OLDPATH
}

function _libstd {
	pushd .

	cd $GCC_BUILD_DIR
	make -j4 all-target-libstdc++-v3
	make install-target-libstdc++-v3

	popd
}

function _build {
	_sysroot
	_binutils
	_gcc
	_mlibc
	_libstd
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

if [ -z "$1" ]; then
	echo "Usage: $0 (clean/prepare/binutils/gcc/mlibc/build/all)"
else
	cd $SPATH
	_$1
fi

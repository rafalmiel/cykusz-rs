#!/bin/bash

set -x -e

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)
SRC_DIR=$CYKUSZ_DIR/sysroot/src
BINUTILS_SRC_DIR=$SRC_DIR/binutils-gdb
GCC_SRC_DIR=$SRC_DIR/gcc
MLIBC_SRC_DIR=$SRC_DIR/mlibc
NYANCAT_SRC_DIR=$SRC_DIR/nyancat
GMP_SRC_DIR=$SRC_DIR/gmp
MPFR_SRC_DIR=$SRC_DIR/mpfr
MPC_SRC_DIR=$SRC_DIR/mpc

BUILD_DIR=$CYKUSZ_DIR/sysroot/build
BINUTILS_BUILD_DIR=$BUILD_DIR/binutils-gdb
BINUTILS_CYKUSZ_BUILD_DIR=$BUILD_DIR/cykusz-binutils-gdb
GCC_BUILD_DIR=$BUILD_DIR/gcc
MLIBC_BUILD_DIR=$BUILD_DIR/mlibc
GMP_BUILD_DIR=$BUILD_DIR/gmp
MPFR_BUILD_DIR=$BUILD_DIR/mpfr
MPC_BUILD_DIR=$BUILD_DIR/mpc


SYSROOT=$CYKUSZ_DIR/sysroot/cykusz
CROSS=$CYKUSZ_DIR/sysroot/cross

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
	fi
}

function _prepare_nyancat {
	if [ ! -d $NYANCAT_SRC_DIR ]; then
		mkdir -p $SRC_DIR
		git clone --depth 1 -b cykusz https://github.com/rafalmiel/nyancat.git $NYANCAT_SRC_DIR
	fi
}

function _prepare_gmp {
	if [ ! -d $GMP_SRC_DIR ]; then
		mkdir -p $SRC_DIR
		git clone --depth 1 -b cykusz https://github.com/rafalmiel/gmp.git $GMP_SRC_DIR
	fi
}

function _prepare_mpfr {
	if [ ! -d $MPFR_SRC_DIR ]; then
		mkdir -p $SRC_DIR
		git clone --depth 1 -b cykusz https://github.com/rafalmiel/mpfr.git $MPFR_SRC_DIR

		pushd .
		cd $MPFR_SRC_DIR
		autoreconf
		popd
	fi
}

function _prepare_mpc {
	if [ ! -d $MPC_SRC_DIR ]; then
		mkdir -p $SRC_DIR
		git clone --depth 1 -b cykusz https://github.com/rafalmiel/mpc.git $MPC_SRC_DIR
	fi
}

function _prepare {
	_prepare_mlibc
	_prepare_binutils
	_prepare_gcc
	_prepare_nyancat
	_prepare_gmp
	_prepare_mpfr
	_prepare_mpc
}

function _sysroot {
	mkdir -p $BUILD_DIR

	rm -rf $MLIBC_BUILD_DIR
	meson setup --cross-file $SPATH/cross-file.ini --prefix $SYSROOT/usr -Dheaders_only=true -Dstatic=true $MLIBC_BUILD_DIR $MLIBC_SRC_DIR
	meson install -C $MLIBC_BUILD_DIR
}

function _binutils {
	_prepare_binutils

	mkdir -p $BINUTILS_BUILD_DIR

	pushd .

	cd $BINUTILS_BUILD_DIR
	$BINUTILS_SRC_DIR/configure --target=x86_64-cykusz --prefix="$CROSS" --with-sysroot=$SYSROOT --disable-werror --disable-gdb

	popd

	make -C $BINUTILS_BUILD_DIR -j4
	make -C $BINUTILS_BUILD_DIR install

}

function _gcc {
	_prepare_gcc

	mkdir -p $GCC_BUILD_DIR

	pushd .

	cd $GCC_BUILD_DIR
	$GCC_SRC_DIR/configure --target=x86_64-cykusz --prefix="$CROSS" --with-sysroot=$SYSROOT --enable-languages=c,c++ --enable-threads=posix

	popd

	make -C $GCC_BUILD_DIR -j4 all-gcc all-target-libgcc
	make -C $GCC_BUILD_DIR install-gcc install-target-libgcc
}

function _mlibc {
	_prepare_mlibc

	mkdir -p $BUILD_DIR

	rm -rf $MLIBC_BUILD_DIR
	meson setup --cross-file $SPATH/cross-file.ini --prefix $SYSROOT/usr -Dheaders_only=false -Dstatic=true $MLIBC_BUILD_DIR $MLIBC_SRC_DIR

	ninja -C $MLIBC_BUILD_DIR
	meson install -C $MLIBC_BUILD_DIR
}

function _libstd {
	make -C $GCC_BUILD_DIR -j4 all-target-libstdc++-v3
	make -C $GCC_BUILD_DIR install-target-libstdc++-v3
}

function _nyancat {
	_prepare_nyancat

	pushd .

	cd $NYANCAT_SRC_DIR/src
	make clean
	CC=x86_64-cykusz-gcc make
	cp nyancat $BUILD_DIR
	make clean

	popd
}

function _gmp {
	_prepare_gmp

	mkdir -p $GMP_BUILD_DIR

	pushd .

	cd $GMP_BUILD_DIR
	$GMP_SRC_DIR/configure --host=x86_64-cykusz --prefix=/usr

	popd

	make -C $GMP_BUILD_DIR
	make -C $GMP_BUILD_DIR DESTDIR=$SYSROOT install
}

function _mpfr {
	_prepare_mpfr

	mkdir -p $MPFR_BUILD_DIR

	pushd .

	cd $MPFR_BUILD_DIR
	$MPFR_SRC_DIR/configure --host=x86_64-cykusz --prefix=/usr

	popd

	make -C $MPFR_BUILD_DIR
	make -C $MPFR_BUILD_DIR DESTDIR=$SYSROOT install
}

function _mpc {
	_prepare_mpc

	mkdir -p $MPC_BUILD_DIR

	pushd .

	cd $MPC_BUILD_DIR
	$MPC_SRC_DIR/configure --host=x86_64-cykusz --prefix=/usr

	popd

	make -C $MPC_BUILD_DIR
	make -C $MPC_BUILD_DIR DESTDIR=$SYSROOT install

}

function _cykusz_binutils {
	_prepare_binutils

	mkdir -p $BINUTILS_CYKUSZ_BUILD_DIR

	pushd .

	cd $BINUTILS_CYKUSZ_BUILD_DIR
	$BINUTILS_SRC_DIR/configure --host=x86_64-cykusz --with-build-sysroot=$SYSROOT --prefix=/usr --disable-werror --disable-gdb

	popd

	make -C $BINUTILS_CYKUSZ_BUILD_DIR -j4
	make -C $BINUTILS_CYKUSZ_BUILD_DIR DESTDIR=$SYSROOT install
}

function _build {
	_sysroot
	_binutils
	_gcc
	_mlibc
	_libstd
	_nyancat
	#_gmp
	#_mpfr
	#_mpc
	_cykusz_binutils
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
	if [ ! -f $CROSS/bin/x86_64-cykusz-gcc ]; then
		_all
	fi
}

if [ -z "$1" ]; then
	echo "Usage: $0 (clean/prepare/binutils/gcc/mlibc/nyancat/check_build/build/all)"
else
	cd $SPATH
	_$1
fi

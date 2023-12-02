#!/bin/bash

set -xe

SPATH=$(dirname $(readlink -f "$0"))
LOGDIR=$SPATH/log

mkdir -p $LOGDIR

echo "Getting linux headers..."
$SPATH/build.sh linux_headers  > $LOGDIR/linux_headers.log 2>&1

echo "Creating sysroot..."
$SPATH/build.sh sysroot  > $LOGDIR/sysroot.log 2>&1

echo "Building libtool..."
$SPATH/build.sh libtool > $LOGDIR/libtool.log 2>&1

echo "Building binutils..."
$SPATH/build.sh binutils > $LOGDIR/binutils.log 2>&1

echo "Building gcc..."
$SPATH/build.sh gcc > $LOGDIR/gcc.log 2>&1

echo "Building dummy_libc..."
$SPATH/build.sh dummy_libc > $LOGDIR/dummy_libc.log 2>&1

echo "Building libgcc..."
$SPATH/build.sh libgcc > $LOGDIR/libgcc.log 2>&1

echo "Building mlibc..."
$SPATH/build.sh mlibc > $LOGDIR/mlibc.log 2>&1

echo "Building libstdc++..."
$SPATH/build.sh libstd > $LOGDIR/libstd.log 2>&1

echo "Building tzdb..."
$SPATH/build.sh cykusz_tzdb > $LOGDIR/cykusz_tzdb.log 2>&1

echo "Cross building binutils..."
$SPATH/build.sh cykusz_binutils > $LOGDIR/cykusz_binutils.log 2>&1

echo "Cross building gcc..."
$SPATH/build.sh cykusz_gcc > $LOGDIR/cykusz_gcc.log 2>&1

echo "Cross building libgcc..."
$SPATH/build.sh cykusz_libgcc > $LOGDIR/cykusz_libgcc.log 2>&1

echo "Cross building libstdc++..."
$SPATH/build.sh cykusz_libstd > $LOGDIR/cykusz_libstd.log 2>&1

echo "Cross building coreutils..."
$SPATH/build.sh cykusz_coreutils > $LOGDIR/cykusz_coreutils.log 2>&1

echo "Cross building nyancat..."
$SPATH/build.sh cykusz_nyancat > $LOGDIR/cykusz_nyancat.log 2>&1

echo "Cross building ncurses..."
$SPATH/build.sh cykusz_ncurses > $LOGDIR/cykusz_ncurses.log 2>&1

echo "Cross building nano..."
$SPATH/build.sh cykusz_nano > $LOGDIR/cykusz_nano.log 2>&1

echo "Cross building doom..."
$SPATH/build.sh cykusz_doom > $LOGDIR/cykusz_doom.log 2>&1

echo "Cross building readline..."
$SPATH/build.sh cykusz_readline > $LOGDIR/cykusz_readline.log 2>&1

echo "Cross building bash..."
$SPATH/build.sh cykusz_bash > $LOGDIR/cykusz_bash.log 2>&1

echo "Cross building zstd..."
$SPATH/build.sh cykusz_zstd > $LOGDIR/cykusz_zstd.log 2>&1

echo "Cross building libffi..."
$SPATH/build.sh cykusz_libffi > $LOGDIR/cykusz_libffi.log 2>&1

echo "Cross building libexpat..."
$SPATH/build.sh cykusz_libexpat > $LOGDIR/cykusz_libexpat.log 2>&1

echo "Cross building libunistring..."
$SPATH/build.sh cykusz_libunistring > $LOGDIR/cykusz_libunistring.log 2>&1

echo "Cross building libiconv..."
$SPATH/build.sh cykusz_libiconv > $LOGDIR/cykusz_libiconv.log 2>&1

echo "Cross building libidn2..."
$SPATH/build.sh cykusz_libidn2 > $LOGDIR/cykusz_libidn2.log 2>&1

echo "Cross building zlib..."
$SPATH/build.sh cykusz_zlib > $LOGDIR/cykusz_zlib.log 2>&1

echo "Cross building pcre2..."
$SPATH/build.sh cykusz_pcre2 > $LOGDIR/cykusz_pcre2.log 2>&1

echo "Cross building libressl..."
$SPATH/build.sh cykusz_libressl > $LOGDIR/cykusz_libressl.log 2>&1

echo "Cross building less..."
$SPATH/build.sh cykusz_less > $LOGDIR/cykusz_less.log 2>&1

echo "Cross building netcat..."
$SPATH/build.sh cykusz_netcat > $LOGDIR/cykusz_netcat.log 2>&1

echo "Cross building wget..."
$SPATH/build.sh cykusz_wget > $LOGDIR/cykusz_wget.log 2>&1

echo "Cross building python..."
$SPATH/build.sh cykusz_python > $LOGDIR/cykusz_python.log 2>&1
#
#echo "Cross building llvm..."
#$SPATH/build.sh cykusz_llvm > $LOGDIR/cykusz_llvm.log 2>&1

echo "Success!"

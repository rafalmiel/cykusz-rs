#!/bin/bash

set -xe

SPATH=$(dirname $(readlink -f "$0"))
LOGDIR=$SPATH/log

mkdir -p $LOGDIR

echo "Creating sysroot..."
$SPATH/build.sh sysroot  > $LOGDIR/sysroot.log 2>&1

echo "Building binutils..."
$SPATH/build.sh binutils > $LOGDIR/binutils.log 2>&1

echo "Building gcc..."
$SPATH/build.sh gcc > $LOGDIR/gcc.log 2>&1

echo "Building mlibc..."
$SPATH/build.sh mlibc > $LOGDIR/mlibc.log 2>&1

echo "Building libgcc..."
$SPATH/build.sh libgcc > $LOGDIR/libgcc.log 2>&1

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

echo "Cross building bash..."
$SPATH/build.sh cykusz_bash > $LOGDIR/cykusz_bash.log 2>&1

echo "Cross building zstd..."
$SPATH/build.sh cykusz_zstd > $LOGDIR/cykusz_zstd.log 2>&1

echo "Cross building llvm..."
$SPATH/build.sh cykusz_llvm > $LOGDIR/cykusz_llvm.log 2>&1

echo "Success!"

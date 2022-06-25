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

echo "Cross building binutils..."
$SPATH/build.sh cykusz_binutils > $LOGDIR/cykusz_binutils.log 2>&1

echo "Cross building gcc..."
$SPATH/build.sh cykusz_gcc > $LOGDIR/cykusz_gcc.log 2>&1

echo "Cross building libgcc..."
$SPATH/build.sh cykusz_libgcc > $LOGDIR/cykusz_libgcc.log 2>&1

echo "Cross building libstdc++..."
$SPATH/build.sh cykusz_libstd > $LOGDIR/cykusz_libstd.log 2>&1

echo "Cross building ncurses..."
$SPATH/build.sh cykusz_ncurses > $LOGDIR/cykusz_ncurses.log 2>&1

echo "Success!"

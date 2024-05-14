#!/bin/bash

set -xe

SPATH=$(dirname $(readlink -f "$0"))
LOGDIR=$SPATH/log

mkdir -p $LOGDIR

echo "Cross building llvm..."
$SPATH/build.sh cykusz_llvm > $LOGDIR/cykusz_llvm.log 2>&1

echo "Success!"

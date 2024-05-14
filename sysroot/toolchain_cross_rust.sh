#!/bin/bash

set -xe

SPATH=$(dirname $(readlink -f "$0"))
LOGDIR=$SPATH/log

mkdir -p $LOGDIR

echo "Cross building rust..."
$SPATH/build.sh rust > $LOGDIR/rust.log 2>&1

echo "Success!"

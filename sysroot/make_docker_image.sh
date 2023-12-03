#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)

docker build -t cykusz-build $CYKUSZ_DIR/sysroot/docker

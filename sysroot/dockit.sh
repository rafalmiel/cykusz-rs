#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)

docker run -u $(id -u ${USER}):$(id -g ${USER}) -v $CYKUSZ_DIR:/home/ck/code/cykusz-rs -v $SPATH:/home/ck/sysroot cykusz-build:latest $*

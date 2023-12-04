#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)

docker buildx build -t cykusz-build $CYKUSZ_DIR/sysroot/cfg/docker

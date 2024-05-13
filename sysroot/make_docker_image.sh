#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)

docker buildx build -t cykusz-build --build-arg uid=$(id -u) --build-arg gid=$(id -g) $CYKUSZ_DIR/sysroot/cfg/docker

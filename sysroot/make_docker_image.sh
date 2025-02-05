#!/bin/bash

set -x

SPATH=$(dirname "$(readlink -f "$0")")
CYKUSZ_DIR=$(realpath $SPATH/..)

docker buildx build -t cykusz-build --build-arg uid="$(id -u $USER)" --build-arg gid="$(id -g $USER)" --build-arg user=$USER --build-arg workdir="$CYKUSZ_DIR"  "$SPATH"/cfg/docker

#!/bin/bash

set -x

SPATH=$(dirname "$(readlink -f "$0")")
CYKUSZ_DIR=$(realpath $SPATH/..)

docker buildx build -t cykusz-build --build-arg uid="$(id -u $(logname))" --build-arg gid="$(id -g $(logname))" --build-arg user=$(logname) --build-arg workdir="$CYKUSZ_DIR"  "$SPATH"/cfg/docker

#!/bin/bash

set -x

SPATH=$(dirname "$(readlink -f "$0")")

sed -i "s#{WORKDIR}#$SPATH#g" $SPATH/cfg/docker/Dockerfile

docker buildx build -t cykusz-build --build-arg uid="$(id -u)" --build-arg gid="$(id -g)" "$SPATH"/cfg/docker

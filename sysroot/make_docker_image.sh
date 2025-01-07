#!/bin/bash

set -x

SPATH=$(dirname "$(readlink -f "$0")")

docker buildx build -t cykusz-build --build-arg uid="$(id -u $(logname))" --build-arg gid="$(id -g $(logname))" "$SPATH"/cfg/docker

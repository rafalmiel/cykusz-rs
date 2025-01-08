#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)

docker run -u $(id -u ${USER}):$(id -g ${USER}) -v $CYKUSZ_DIR:$CYKUSZ_DIR cykusz-build:latest $*

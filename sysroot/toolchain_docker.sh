#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))

$SPATH/dockit.sh ./toolchain.sh

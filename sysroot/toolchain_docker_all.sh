#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))

$SPATH/dockit.sh ./toolchain.sh
$SPATH/dockit.sh ./toolchain_cross_llvm.sh
$SPATH/dockit.sh ./toolchain_cross_rust.sh


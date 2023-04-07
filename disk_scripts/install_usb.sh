#!/bin/bash

set -xe

dev=$1

mkdir -p mnt
mount "$dev" mnt

cp -f build/kernel-x86_64.bin mnt/kernel.bin

umount mnt

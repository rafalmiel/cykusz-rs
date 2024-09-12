#!/bin/bash

set -xe

dev=$1

mkdir -p mnt
sudo mount "$dev" mnt
sudo chown $USER:$USER mnt

cp -f build/kernel-x86_64.bin mnt/kernel.bin

sudo umount mnt

#!/bin/bash

set -e

dd if=/dev/zero of=disk.img count=240 bs=$((1024*1024))
chown $1:$1 disk.img

parted disk.img mktable msdos -s
parted disk.img mkpart primary ext2 2048s 133119s # 32MB
parted disk.img mkpart primary ext2 133120s 362495s # 112MB
parted disk.img mkpart primary ext2 362496s 428033s # 64
parted disk.img set 1 boot on

losetup -D
losetup -P /dev/loop0 disk.img
mkfs.ext2 /dev/loop0p1
mkfs.ext2 /dev/loop0p2
mkfs.ext2 /dev/loop0p3
losetup -D

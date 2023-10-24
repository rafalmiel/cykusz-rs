#!/bin/bash

set -e

dd if=/dev/zero of=disk.img count=$((1024*4 + 64)) bs=$((1024*1024))
chown $1:$1 disk.img

parted disk.img mktable msdos -s
parted disk.img mkpart primary ext2 2048s 64MiB
parted -- disk.img mkpart primary ext2 64Mib 3GiB # 112MB
parted -- disk.img mkpart primary ext2 3GiB 3.5Gib # 64
parted disk.img set 1 boot on

losetup -D
losetup -P /dev/loop0 disk.img
mkfs.ext2 /dev/loop0p1
mkfs.ext2 /dev/loop0p2
mkfs.ext2 /dev/loop0p3
losetup -D

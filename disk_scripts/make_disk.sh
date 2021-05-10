#!/bin/bash

set -e

dd if=/dev/zero of=disk.img count=512 bs=$((1024*1024))
chown $1:$1 disk.img

parted disk.img mktable msdos -s
parted disk.img mkpart primary ext2 2048s 64MiB
parted disk.img mkpart primary ext2 64Mib 300MiB # 112MB
parted -- disk.img mkpart primary ext2 300MiB -1s # 64
parted disk.img set 1 boot on

losetup -D
losetup -P /dev/loop0 disk.img
mkfs.ext2 /dev/loop0p1
mkfs.ext2 /dev/loop0p2
mkfs.ext2 /dev/loop0p3
losetup -D

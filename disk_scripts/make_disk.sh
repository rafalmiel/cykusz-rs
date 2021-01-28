#!/bin/bash

set -e

dd if=/dev/zero of=disk.img count=192 bs=$((1024*1024))

parted disk.img mktable msdos -s
parted disk.img mkpart primary ext2 2048s 32767s
parted disk.img mkpart primary ext2 32768s 262143s
parted disk.img mkpart primary ext2 262144s 393215s
parted disk.img set 1 boot on

losetup -D
losetup -P /dev/loop0 disk.img
mkfs.ext2 /dev/loop0p1
mkfs.ext2 /dev/loop0p2
mkfs.ext2 /dev/loop0p3
losetup -D

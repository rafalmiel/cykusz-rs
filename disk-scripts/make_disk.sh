#!/bin/bash

set -e

dd if=/dev/zero of=disk.img count=$((1024*5 + 64)) bs=$((1024*1024))

parted disk.img mktable msdos -s
parted disk.img mkpart primary ext2 2048s 64MiB
parted -- disk.img mkpart primary ext2 64Mib 4GiB # 112MB
parted -- disk.img mkpart primary ext2 4GiB 4.5Gib # 64
parted disk.img set 1 boot on

lo=$(losetup -f)
losetup -P $lo disk.img
sudo -u $USER mkfs.ext2 "$lo"p1
sudo -u $USER mkfs.ext2 "$lo"p2
sudo -u $USER mkfs.ext2 "$lo"p3
losetup -d $lo

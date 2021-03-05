#!/bin/bash

losetup -D
losetup -P /dev/loop0 disk.img

mkdir -p mnt
mount /dev/loop0p1 mnt

cp -f build/isofiles/boot/kernel.bin mnt/
cp -f build/isofiles/boot/grub/grub.cfg mnt/grub/

umount mnt

mount /dev/loop0p2 mnt
mkdir -p mnt/bin
cp -f build/isofiles/boot/program2 mnt/bin/shell
umount mnt

losetup -D

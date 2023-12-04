#!/bin/bash

losetup -D
losetup -P /dev/loop0 disk.img

mkdir -p mnt
mount /dev/loop0p1 mnt

grub-install --root-directory=mnt --boot-directory=mnt --no-floppy --target=i386-pc --modules="normal part_msdos ext2 multiboot2" /dev/loop0

umount mnt

mount /dev/loop0p2 mnt

mkdir -p mnt/boot
mkdir -p mnt/dev
mkdir -p mnt/bin

umount mnt

losetup -D

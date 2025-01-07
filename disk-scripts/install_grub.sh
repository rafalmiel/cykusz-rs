#!/bin/bash

lo=$(losetup -f)
losetup -P $lo disk.img

mkdir -p mnt
sudo mount "$lo"p1 mnt
sudo chown $USER:$USER mnt

grub-install --root-directory=mnt --boot-directory=mnt --no-floppy --target=i386-pc --modules="normal part_msdos ext2 multiboot2" $lo

sudo umount mnt

sudo mount "$lo"p2 mnt
sudo chown $USER:$USER mnt

mkdir -p mnt/boot
mkdir -p mnt/dev
mkdir -p mnt/bin

sudo umount mnt

losetup -d $lo

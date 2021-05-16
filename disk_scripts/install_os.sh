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
cp -f target/x86_64-unknown-none-gnu/release/init mnt/bin/init
cp -f target/x86_64-unknown-none-gnu/release/shell mnt/bin/shell
cp -f sysroot/build/hello mnt/bin/hello
cp -f sysroot/build/stack mnt/bin/stack
cp -f sysroot/build/nyancat mnt/bin/nyancat
cp -f sysroot/build/ttytest mnt/bin/ttytest
cp -r sysroot/cykusz/usr mnt/
#cp sysroot/cykusz/usr/bin/{readelf,objdump,nm,strings,size} mnt/bin/
umount mnt

losetup -D

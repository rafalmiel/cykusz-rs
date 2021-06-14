#!/bin/bash

losetup -D
losetup -P /dev/loop0 disk.img

mkdir -p mnt
mount /dev/loop0p1 mnt

cp -f build/isofiles/boot/kernel.bin mnt/
cp -f build/isofiles/boot/grub/grub.cfg mnt/grub/

umount mnt

PROGS="test testcpp hello stack nyancat ttytest fork"

mount /dev/loop0p2 mnt
mkdir -p mnt/bin
cp -f target/x86_64-unknown-none-gnu/release/init mnt/bin/init
cp -f target/x86_64-unknown-none-gnu/release/shell mnt/bin/shell

for prog in $PROGS; do
	cp -f sysroot/build/$prog mnt/bin/$prog
done

cp -r sysroot/cykusz/usr mnt/

#cp -r sysroot/cross/x86_64-cykusz/lib/* mnt/usr/lib/
#cp sysroot/test.c mnt/
#cp sysroot/stack.c mnt/
#cp sysroot/test.cpp mnt/
#cp sysroot/hello.cpp mnt/
##cp sysroot/cykusz/usr/bin/{readelf,objdump,nm,strings,size} mnt/bin/
umount mnt

losetup -D

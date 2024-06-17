#!/bin/bash

losetup -D
losetup -P /dev/loop0 disk.img

mkdir -p mnt
mount /dev/loop0p1 mnt

cp -f build/isofiles/boot/kernel.bin mnt/
cp -f build/isofiles/boot/grub/grub.cfg mnt/grub/

sed -i "s/{ROOT_UUID}/$(blkid -s UUID -o value /dev/loop0p2)/g" mnt/grub/grub.cfg

umount mnt

PROGS="test testcpp hello stack nyancat ttytest fork poweroff stat fbdoom doom1.wad open_sleep"
RUST_PROGS="init shell mount umount unixsocket-server unixsocket-client forktest"

mount /dev/loop0p2 mnt
mkdir -p mnt/bin

for prog in $PROGS; do
	cp -f sysroot/build/$prog mnt/bin/$prog
done

for prog in $RUST_PROGS; do
    cp -f userspace/target/x86_64-unknown-cykusz/release/$prog mnt/bin/$prog
done

rsync -a sysroot/cykusz/usr mnt/
rsync -a sysroot/cykusz/etc mnt/

mkdir -p mnt/etc
mkdir -p mnt/home
echo $(blkid -s UUID -o value /dev/loop0p1) /boot > mnt/etc/fstab
echo $(blkid -s UUID -o value /dev/loop0p3) /home >> mnt/etc/fstab

umount mnt

losetup -D

#!/bin/bash

set -x

losetup -D
losetup -P /dev/loop0 disk.img

mkdir -p mnt
sudo mount /dev/loop0p1 mnt
sudo chown $USER:$USER mnt

if [ "$1" == "debug" ]
then
    KERNEL=kernel-x86_64-g.bin
else
    KERNEL=kernel-x86_64.bin
fi

cp -f build/$KERNEL mnt/kernel.bin
cp -f cykusz-rs/src/arch/x86_64/asm/grub.cfg mnt/grub/

sed -i "s/{ROOT_UUID}/$(blkid -s UUID -o value /dev/loop0p2)/g" mnt/grub/grub.cfg
sed -i "s/{LOGS}/$CYKUSZ_LOGS/g" mnt/grub/grub.cfg

RUST_PROG_MODE=release

if [ "$#" -ge 1 ]
then
    RUST_PROG_MODE=$1
fi

sudo umount mnt

PROGS="test testcpp hello stack nyancat ttytest fork poweroff stat fbdoom doom1.wad open_sleep"
RUST_PROGS="init shell mount umount unixsocket-server unixsocket-client forktest mprotecttest playwav playmidi threads sound-daemon"

sudo mount /dev/loop0p2 mnt
sudo chown -R $USER:$USER mnt
mkdir -p mnt/bin

for prog in $PROGS; do
	cp -f sysroot/build/$prog mnt/bin/$prog
done

for prog in $RUST_PROGS; do
    cp -f userspace/target/x86_64-unknown-cykusz/$RUST_PROG_MODE/$prog mnt/bin/$prog
done

cp sysroot/assets/imperial.wav mnt/
cp sysroot/assets/D_E1M1.mid mnt/

if ! [ -f sysroot/assets/FluidR3_GM.sf2 ]; then
    pushd .
    cd sysroot/assets || exit 1
    wget https://keymusician01.s3.amazonaws.com/FluidR3_GM.zip
    unzip FluidR3_GM.zip
    sync
    popd || exit 1
fi

cp sysroot/assets/FluidR3_GM.sf2 mnt/

rsync -a sysroot/cykusz/usr mnt/
rsync -a sysroot/cykusz/etc mnt/

mkdir -p mnt/etc
mkdir -p mnt/home
echo "$(blkid -s UUID -o value /dev/loop0p1)" /boot > mnt/etc/fstab
echo "$(blkid -s UUID -o value /dev/loop0p3)" /home >> mnt/etc/fstab

sudo umount mnt

losetup -D

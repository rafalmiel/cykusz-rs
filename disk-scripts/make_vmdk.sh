#!/bin/bash

rm ./disk.vmdk
losetup -D
losetup /dev/loop0 ./disk.img
VBoxManage internalcommands createrawvmdk -filename sysroot/cfg/disk.vmdk -rawdisk /dev/loop0
losetup -D

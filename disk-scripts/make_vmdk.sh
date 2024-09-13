#!/bin/bash

VBoxManage storageattach cykusz --storagectl AHCI --port 0 --medium emptydrive
VBoxManage closemedium sysroot/cfg/disk.vmdk --delete

losetup -D
losetup /dev/loop0 ./disk.img
VBoxManage createmedium disk --filename sysroot/cfg/disk.vmdk --format=VMDK --variant RawDisk --property RawDrive=/dev/loop0
losetup -D

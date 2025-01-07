#!/bin/bash

VBoxManage storageattach cykusz --storagectl AHCI --port 0 --medium emptydrive
VBoxManage closemedium sysroot/cfg/disk.vmdk --delete

lo=$(losetup -f)
losetup $lo ./disk.img
VBoxManage createmedium disk --filename sysroot/cfg/disk.vmdk --format=VMDK --variant RawDisk --property RawDrive=$lo
losetup -d $lo

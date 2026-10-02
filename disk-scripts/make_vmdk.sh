#!/bin/bash

VBoxManage storageattach cykusz --storagectl AHCI --port 0 --medium emptydrive
VBoxManage closemedium sysroot/cfg/disk.vmdk --delete

lo=$(losetup -f)
trap 'losetup -d $lo 2>/dev/null || true' EXIT
losetup $lo ./disk.img
VBoxManage createmedium disk --filename sysroot/cfg/disk.vmdk --format=VMDK --variant RawDisk --property RawDrive=$lo

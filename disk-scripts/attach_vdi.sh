#!/bin/bash

VBoxManage storageattach cykusz --storagectl AHCI --port 0 --medium emptydrive
VBoxManage closemedium sysroot/cfg/disk.vmdk
VBoxManage storageattach cykusz --storagectl AHCI --medium sysroot/cfg/disk.vmdk --port 0 --type hdd

#!/bin/bash

VBoxManage storageattach cykusz --storagectl AHCI --port 0 --medium emptydrive
VBoxManage closemedium disk.vdi
VBoxManage storageattach cykusz --storagectl AHCI --medium disk.vdi --port 0 --type hdd

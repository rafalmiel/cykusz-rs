#!/bin/bash

VBoxManage storageattach cykusz --storagectl AHCI --medium sysroot/cfg/disk.vmdk --port 0 --type hdd

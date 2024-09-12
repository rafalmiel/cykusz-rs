#!/bin/bash

rm ./disk.vmdk
VBoxManage internalcommands createrawvmdk -filename disk.vmdk -rawdisk /dev/loop0

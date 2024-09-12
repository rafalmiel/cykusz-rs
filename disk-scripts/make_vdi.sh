#!/bin/bash

rm ./disk.vdi
VBoxManage convertfromraw disk.img disk.vdi --format vdi

#!/bin/bash

user=$USER

sudo disk_scripts/make_disk.sh $user
sleep 1
sudo disk_scripts/install_grub.sh

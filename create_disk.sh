#!/bin/bash

user=$USER

sudo disk_scripts/make_disk.sh $user
sudo disk_scripts/install_grub.sh

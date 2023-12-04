#!/bin/bash

user=$USER

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)

sudo $CYKUSZ_DIR/disk-scripts/make_disk.sh $user
sleep 1
sudo $CYKUSZ_DIR/disk-scripts/install_grub.sh

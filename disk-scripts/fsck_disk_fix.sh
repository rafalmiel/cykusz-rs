#!/bin/bash

set -e

losetup -D
losetup -P /dev/loop0 disk.img

sudo -u $USER fsck.ext2 /dev/loop0p1 -f -v -y
sudo -u $USER fsck.ext2 /dev/loop0p2 -f -v -y
sudo -u $USER fsck.ext2 /dev/loop0p3 -f -v -y

losetup -D

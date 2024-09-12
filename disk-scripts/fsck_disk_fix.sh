#!/bin/bash

set -e

losetup -D
losetup -P /dev/loop0 disk.img

sudo -u $USER fsck.ext2 /dev/loop0p1 -f -v -p
sudo -u $USER fsck.ext2 /dev/loop0p2 -f -v -p
sudo -u $USER fsck.ext2 /dev/loop0p3 -f -v -p

losetup -D

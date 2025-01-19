#!/bin/bash

set -e

lo=$(losetup -f)
losetup -P $lo disk.img

sudo fsck.ext2 "$lo"p1 -f -v -y
sudo fsck.ext2 "$lo"p2 -f -v -y
sudo fsck.ext2 "$lo"p3 -f -v -y

losetup -d $lo

#!/bin/bash

set -xe

lo=$(losetup -f)
u=$(logname)
losetup -P $lo disk.img

sudo fsck.ext2 "$lo"p1 -f -v -n
sudo fsck.ext2 "$lo"p2 -f -v -n
sudo fsck.ext2 "$lo"p3 -f -v -n

losetup -d $lo

#!/bin/bash

set -e

lo=$(losetup -f)
losetup -P $lo disk.img

sudo -u $USER fsck.ext2 "$lo"p1 -f -v -n
sudo -u $USER fsck.ext2 "$lo"p2 -f -v -n
sudo -u $USER fsck.ext2 "$lo"p3 -f -v -n

losetup -d $lo

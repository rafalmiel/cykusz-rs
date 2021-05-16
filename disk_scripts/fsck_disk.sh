#!/bin/bash

set -e

losetup -D
losetup -P /dev/loop0 disk.img

fsck.ext2 /dev/loop0p1 -f -v -n
fsck.ext2 /dev/loop0p2 -f -v -n
#fsck.ext2 /dev/loop0p3 -f -v -n

losetup -D

#!/bin/bash

set -xe

lo=$(losetup -f)
trap 'losetup -d $lo 2>/dev/null || true' EXIT
losetup -P $lo disk.img
udevadm settle

fsck.ext2 "$lo"p1 -f -v -n
fsck.ext2 "$lo"p2 -f -v -n
fsck.ext2 "$lo"p3 -f -v -n

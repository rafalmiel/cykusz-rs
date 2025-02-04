#!/bin/bash

set -ex

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)

$CYKUSZ_DIR/disk-scripts/make_disk.sh
sync
$CYKUSZ_DIR/disk-scripts/install_grub.sh

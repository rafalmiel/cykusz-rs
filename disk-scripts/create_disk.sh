#!/bin/bash

SPATH=$(dirname $(readlink -f "$0"))
CYKUSZ_DIR=$(realpath $SPATH/..)

$CYKUSZ_DIR/disk-scripts/make_disk.sh
sleep 1
$CYKUSZ_DIR/disk-scripts/install_grub.sh

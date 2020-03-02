#!/bin/bash

cd acpica/acpica && git reset --hard origin/master && cd -
rm acpica/acpica_patched || true
git commit -a $*

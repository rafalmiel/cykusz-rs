#!/bin/bash

find ./cykusz-rs/src -type f -exec sed --in-place 's/[[:space:]]\+$//' {} \+

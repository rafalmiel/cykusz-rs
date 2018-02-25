#!/bin/bash

find ./src -type f -exec sed --in-place 's/[[:space:]]\+$//' {} \+

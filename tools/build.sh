#!/bin/bash
set -e
profile=${1:-debug}
if [ "$profile" = "release" ]; then flags="--release"; else flags=""; fi
cargo build $flags --bins
mkdir -p python/xhtmlmd.data/scripts
cp target/$profile/xhtmlmd python/xhtmlmd.data/scripts/

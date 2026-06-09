#!/bin/bash
set -e
profile=${1:-debug}
if [ "$profile" = "release" ]; then flags="--release"; else flags=""; fi
cargo build $flags --bins
mkdir -p python/xhtml_md_parser.data/scripts
cp target/$profile/xhtml-md python/xhtml_md_parser.data/scripts/

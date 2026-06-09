#!/bin/bash
set -e
cargo test
tools/build.sh
maturin develop
pytest -q

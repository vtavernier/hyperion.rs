#!/bin/bash

set -euo pipefail

ROOT=release/hyperion.rs

# Create directory structure
mkdir -p $ROOT/bin \
         $ROOT/share/hyperion

# Copy assets
cp target/$1/release/hyperiond $ROOT/bin/hyperiond-rs
cp -rv ext/hyperion.ng/assets/webconfig $ROOT/share/hyperion

# Create archive
cd release
tar cJvf hyperion.rs-$1.tar.xz hyperion.rs

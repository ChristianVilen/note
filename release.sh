#!/bin/bash
set -e
echo "Building and installing note..."
cargo install --path .
echo "Installed: v$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
echo "Done!"

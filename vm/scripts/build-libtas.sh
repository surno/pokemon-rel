#!/usr/bin/env bash
set -euo pipefail

SRC_DIR=/opt/src
INSTALL_DIR=/opt/libtas
mkdir -p "$SRC_DIR" "$INSTALL_DIR"

git clone --depth 1 https://github.com/clementgallet/libTAS.git "$SRC_DIR/libtas" --branch v1.4.6

cd "$SRC_DIR/libtas/"

# Build libTAS
./build.sh


ln -sf "$INSTALL_DIR/bin/libTAS" /usr/local/bin/libtas
#!/usr/bin/env bash
set -euo pipefail

# Where we’ll stage source & install
SRC_DIR=/opt/src
INSTALL_DIR=/opt/melonds
mkdir -p "$SRC_DIR" "$INSTALL_DIR"

# Build
git clone --depth 1 https://github.com/melonDS-emu/melonDS.git "$SRC_DIR/melonds"
cmake -S "$SRC_DIR/melonds" -B "$SRC_DIR/melonds/build" \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX="$INSTALL_DIR" \
      -DUSE_SYSTEM_SDL2=ON -Wno-dev
cmake --build "$SRC_DIR/melonds/build" --target install -j"$(nproc)"

# Symlink binary into PATH
ln -sf "$INSTALL_DIR/bin/melonds" /usr/local/bin/melonds
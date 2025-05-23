#!/usr/bin/env bash
set -euo pipefail

SRC_DIR=/opt/src
INSTALL_DIR=/opt/libtas
mkdir -p "$SRC_DIR" "$INSTALL_DIR"

git clone --depth 1 https://github.com/clementgallet/libTAS.git "$SRC_DIR/libtas"
cmake -S "$SRC_DIR/libtas" -B "$SRC_DIR/libtas/build" \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX="$INSTALL_DIR" \
      -DBUILD_SHARED_LIBS=OFF -Wno-dev
cmake --build "$SRC_DIR/libtas/build" --target install -j"$(nproc)"

ln -sf "$INSTALL_DIR/bin/libTAS" /usr/local/bin/libtas
#!/usr/bin/env bash
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get upgrade -y

# Generic toolchain & helpers
apt-get install -y --no-install-recommends \
  build-essential git cmake pkg-config ninja-build \
  curl unzip ca-certificates sudo

# melonDS deps
apt-get install -y --no-install-recommends \
  libsdl2-dev libfmt-dev libepoxy-dev libpcap-dev \
  libpulse-dev libavcodec-dev libavformat-dev libswscale-dev \
  zlib1g-dev

# libTAS deps (SDL2 + Qt)
apt-get install -y --no-install-recommends \
  qtbase5-dev qtbase5-private-dev qttools5-dev libqt5svg5-dev \
  libsdl2-ttf-dev libsdl2-image-dev libsdl2-mixer-dev

# tiny clean-up
apt-get clean
rm -rf /var/lib/apt/lists/*
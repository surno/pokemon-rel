#!/usr/bin/env bash
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get upgrade -y

# Generic toolchain & helpers
apt-get install -y --no-install-recommends \
  build-essential git cmake pkg-config ninja-build \
  curl unzip ca-certificates sudo libenet-dev extra-cmake-modules

# melonDS deps
apt-get install -y --no-install-recommends \
  libsdl2-dev libfmt-dev libepoxy-dev libpcap-dev \
  libpulse-dev libavcodec-dev libavformat-dev libswscale-dev \
  zlib1g-dev libgl1-mesa-dev libarchive-dev

# libTAS deps (SDL2 + Qt 6)
apt-get install -y --no-install-recommends \
  qt6-base-dev qt6-base-dev-tools qt6-base-private-dev qt6-tools-dev qt6-tools-dev-tools qt6-svg-dev \
  libsdl2-ttf-dev libsdl2-image-dev libsdl2-mixer-dev qt6-multimedia-dev libqt6opengl6-dev \
  extra-cmake-modules qt6-declarative-dev

# tiny clean-up
apt-get clean
rm -rf /var/lib/apt/lists/*
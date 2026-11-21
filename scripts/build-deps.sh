#!/bin/sh
# Builds x264, Opus and (on non-macOS) libyuv as static, position-independent libraries
# into $1, so libvoxl.so/.dylib carries them and needs no system copies.
# Usage: scripts/build-deps.sh <prefix>
set -eu
PREFIX=$(mkdir -p "$1" && cd "$1" && pwd)
SRC=$(mktemp -d)
JOBS=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)
export CFLAGS="-fPIC -O2" CXXFLAGS="-fPIC -O2"

# x264: the stable branch (this repo's patches/x264 are for the nix build and are not applied).
git clone --depth 1 -b stable https://code.videolan.org/videolan/x264.git "$SRC/x264"
(cd "$SRC/x264" && ./configure --prefix="$PREFIX" --enable-static --enable-pic --disable-cli --disable-opencl \
  && make -j"$JOBS" && make install)

git clone --depth 1 -b v1.5.2 https://github.com/xiph/opus.git "$SRC/opus"
cmake -S "$SRC/opus" -B "$SRC/opus/b" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$PREFIX" \
  -DCMAKE_INSTALL_LIBDIR=lib -DBUILD_SHARED_LIBS=OFF -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  -DOPUS_BUILD_PROGRAMS=OFF -DOPUS_BUILD_TESTING=OFF
cmake --build "$SRC/opus/b" -j"$JOBS" && cmake --install "$SRC/opus/b"

# macOS captures NV12 and doesn't use libyuv (see build.rs).
if [ "$(uname -s)" != Darwin ]; then
  git clone https://chromium.googlesource.com/libyuv/libyuv "$SRC/libyuv"
  cmake -S "$SRC/libyuv" -B "$SRC/libyuv/b" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DBUILD_SHARED_LIBS=OFF -DCMAKE_DISABLE_FIND_PACKAGE_JPEG=ON
  cmake --build "$SRC/libyuv/b" -j"$JOBS" && cmake --install "$SRC/libyuv/b"
  # libyuv always installs a shared copy too, which the linker would prefer.
  rm -f "$PREFIX"/lib*/libyuv.so*
  # install may write lib64 or lib; build.rs links libyuv from the default path.
  [ -d "$PREFIX/lib64" ] && cp -n "$PREFIX"/lib64/libyuv.a "$PREFIX/lib/" || true
fi
rm -rf "$SRC"
ls -l "$PREFIX/lib"

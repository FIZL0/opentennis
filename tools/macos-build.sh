#!/usr/bin/env bash
# M162: cross-build hst for Apple Silicon Macs on Linux: clang + rustup's ld64.lld against MacOSX.sdk from
# tools/ios-sdk.py (the same Xcode .xip as iOS; tools/ios-build.sh is the iPhone twin of this).
# Usage: tools/macos-build.sh [cargo args…]   (default: build --release -p hst)
set -euo pipefail
sdk=${HST_MACOS_SDK:-$HOME/.local/share/hst-ios/MacOSX.sdk}
[ -d "$sdk" ] || { echo "no SDK at $sdk: run tools/ios-sdk.py Xcode.xip (plan/HUMAN.md: download Xcode.xip into ~/Downloads)" >&2; exit 1; }
t=aarch64-apple-darwin
rustc --print target-libdir --target $t 2>/dev/null | xargs test -d || { echo "needs (once): rustup target add $t" >&2; exit 1; }
lld=$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/ld64.lld
min=11.0  # Big Sur: the first macOS on Apple Silicon, and Rust's floor for this target
export SDKROOT=$sdk MACOSX_DEPLOYMENT_TARGET=$min
export CC_aarch64_apple_darwin=clang CXX_aarch64_apple_darwin=clang++
export CFLAGS_aarch64_apple_darwin="-target arm64-apple-macos$min -isysroot $sdk"
export CXXFLAGS_aarch64_apple_darwin=$CFLAGS_aarch64_apple_darwin
export CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=clang
export CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS="-C link-arg=-target -C link-arg=arm64-apple-macos$min -C link-arg=-isysroot -C link-arg=$sdk -C link-arg=-fuse-ld=lld -C link-arg=--ld-path=$lld"
[ $# -gt 0 ] || set -- build --release -p hst
exec cargo "$1" --target $t "${@:2}"

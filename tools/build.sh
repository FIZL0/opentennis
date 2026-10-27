#!/bin/bash
# tools/build.sh [linux|linux-aarch64|windows|ios|android|all] — optimised (release) builds of the game into dist/:
#   dist/linux/opentennis        run: ./opentennis "Hot Shots Tennis (USA).iso"
#   dist/linux-aarch64/OpenTennis/  ARM Linux (M154, tools/linux-aarch64.sh): opentennis + OpenTennis.sh, the launcher a
#                         handheld's Ports menu runs (the folder goes in ROCKNIX's roms/ports/)
#   dist/windows/opentennis.exe  run: drop the ISO on opentennis.exe, or `opentennis.exe "Hot Shots Tennis (USA).iso"`
#   dist/ios/opentennis-ios.ipa  unsigned (M91): the player signs it (AltStore, Sideloadly…); the ISO goes in its Files folder.
#                         Needs the iOS SDK once: tools/ios-sdk.py Xcode_*.xip (see tools/ios-build.sh)
#   dist/opentennis.apk   Android (tools/apk.sh: SDK/NDK setup there); the player picks the ISO on first launch.
# mods/ and settings.txt live in the per-user data folder (~/.local/share/hst, %APPDATA%\hst), not beside the binary. No game data goes into dist/.
# Windows is cross-compiled with MinGW. One-time setup (Arch):
#   sudo pacman -S rustup mingw-w64-gcc     # rustup replaces the `rust` package
#   rustup default stable && rustup target add x86_64-pc-windows-gnu
# Linux is built inside Valve's Steam Runtime "sniper" SDK container (glibc 2.31, Debian 11), so it runs on every
# SteamOS 3.x / Steam Deck (3.5 ships glibc 2.37, 3.7 2.41) and inside the Steam Runtime. One-time setup:
#   sudo pacman -S docker && sudo systemctl enable --now docker && sudo usermod -aG docker $USER   # then log in again
# The container's Rust toolchain and cargo cache live in ~/.cache/hst-steamrt; its build in target/steamrt.
set -e
cd "$(dirname "$0")/.."
what=${1:-linux}

sdk=registry.gitlab.steamos.cloud/steamrt/sniper/sdk:latest

# M22: the commit every platform's build is stamped with (crates/hst/build.rs: the version corner and the online build
# id, which must be the same on all of them); the Linux container can't read a worktree's git
if [ -z "$HST_COMMIT" ]; then
    HST_COMMIT=$(git rev-parse HEAD | cut -c1-7)
    [ -z "$(git status --porcelain --untracked-files=no)" ] || HST_COMMIT=$HST_COMMIT+
fi
export HST_COMMIT

linux() {
    if ! docker info >/dev/null 2>&1; then
        echo "linux build needs docker (once): sudo pacman -S docker; sudo systemctl enable --now docker; sudo usermod -aG docker \$USER; log in again" >&2
        exit 1
    fi
    mkdir -p ~/.cache/hst-steamrt
    docker run --rm -u "$(id -u):$(id -g)" -v "$PWD:/src" -v ~/.cache/hst-steamrt:/cache -w /src \
        -e HST_VERSION -e HST_COMMIT -e HOME=/cache -e CARGO_HOME=/cache/cargo -e RUSTUP_HOME=/cache/rustup -e CARGO_TARGET_DIR=/src/target/steamrt \
        $sdk sh -ec '
            [ -x /cache/cargo/bin/cargo ] || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --no-modify-path
            /cache/cargo/bin/cargo build --release -p hst --features ship'
    rm -rf dist/linux && mkdir -p dist/linux  # the tarball is the whole folder: nothing stale in it
    strip -o dist/linux/opentennis target/steamrt/release/hst
    # the icon on Wayland/X11 docks (the window's app id is "opentennis"): opentennis.desktop into ~/.local/share/applications, opentennis.png into
    # ~/.local/share/icons, opentennis on the PATH
    cp crates/hst/icon/hst.png dist/linux/opentennis.png
    printf '[Desktop Entry]\nType=Application\nName=OpenTennis\nExec=opentennis\nIcon=opentennis\nStartupWMClass=opentennis\nCategories=Game;\n' > dist/linux/opentennis.desktop
    local need glibc=2.31
    need=$(objdump -T dist/linux/opentennis | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)
    echo "dist/linux/opentennis needs $need (sniper has GLIBC_$glibc)"
    if [ "$(printf '%s\n' "${need#GLIBC_}" $glibc | sort -V | tail -1)" != $glibc ]; then
        echo "dist/linux/opentennis needs $need, newer than GLIBC_$glibc: won't start on SteamOS" >&2
        exit 1
    fi
}

# M154: cross-compiled in a Debian 11 container (glibc 2.31, like sniper's), its cross compiler and arm64 libraries set up
# once into the local image hst-linux-aarch64; its Rust toolchain and cargo cache in ~/.cache/hst-linux-aarch64, its
# build in target/linux-aarch64
linux_aarch64() {
    if ! docker info >/dev/null 2>&1; then
        echo "linux-aarch64 build needs docker (see linux above)" >&2
        exit 1
    fi
    local img=hst-linux-aarch64
    if ! docker image inspect $img >/dev/null 2>&1; then
        docker rm -f $img-setup >/dev/null 2>&1 || true
        docker run -i --platform linux/amd64 --name $img-setup debian:bullseye sh -s setup < tools/linux-aarch64.sh
        docker commit $img-setup $img >/dev/null && docker rm $img-setup >/dev/null
    fi
    mkdir -p ~/.cache/hst-linux-aarch64
    docker run --rm -u "$(id -u):$(id -g)" -v "$PWD:/src" -v ~/.cache/hst-linux-aarch64:/cache -w /src \
        -e HST_VERSION -e HST_COMMIT -e HOME=/cache -e CARGO_HOME=/cache/cargo -e RUSTUP_HOME=/cache/rustup -e CARGO_TARGET_DIR=/src/target/linux-aarch64 \
        $img sh -ec '
            [ -x /cache/cargo/bin/rustup ] || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --default-toolchain none --no-modify-path
            PATH=/cache/cargo/bin:$PATH sh tools/linux-aarch64.sh build'
}

windows() {
    local t=x86_64-pc-windows-gnu
    if ! command -v x86_64-w64-mingw32-gcc >/dev/null || ! rustc --print target-libdir --target $t 2>/dev/null | xargs test -d; then
        echo "windows build needs (once): sudo pacman -S rustup mingw-w64-gcc; rustup default stable; rustup target add $t" >&2
        exit 1
    fi
    # crt-static: no MinGW runtime DLLs to ship beside the .exe
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc RUSTFLAGS="-C target-feature=+crt-static" \
        cargo build --release -p hst --features ship --target $t
    rm -rf dist/windows && mkdir -p dist/windows
    x86_64-w64-mingw32-strip -o dist/windows/opentennis.exe target/$t/release/hst.exe
    echo "dist/windows/opentennis.exe"
}

case $what in
    linux) linux ;;
    linux-aarch64) linux_aarch64 ;;
    windows) windows ;;
    ios) mkdir -p dist/ios && tools/ios-ipa.sh dist/ios/opentennis-ios.ipa ;;
    android) tools/apk.sh ;;
    all) linux; linux_aarch64; windows; mkdir -p dist/ios && tools/ios-ipa.sh dist/ios/opentennis-ios.ipa; tools/apk.sh ;;
    *) echo "usage: tools/build.sh [linux|linux-aarch64|windows|ios|android|all]" >&2; exit 2 ;;
esac

#!/bin/sh
# tools/linux-aarch64.sh setup|build — M154: the ARM Linux game (aarch64-unknown-linux-gnu: ROCKNIX handhelds like the
# RG34XX SP, a Raspberry Pi), cross-compiled inside a Debian 11 container (glibc 2.31: older than any handheld
# firmware still in use). tools/build.sh linux-aarch64 and pages.yml run it; not meant to be run by hand.
#   setup  (as root, once per container) the cross compiler and the arm64 libraries the game links (ALSA, udev)
#   build  (from the checkout) builds into $CARGO_TARGET_DIR (target/linux-aarch64), then dist/linux-aarch64/OpenTennis/:
#          opentennis, OpenTennis.sh (the Ports menu launcher: the folder goes in roms/ports/), opentennis.png/.desktop
set -e
t=aarch64-unknown-linux-gnu
glibc=2.31
case $1 in
setup)
    # Debian 11 is past its end of life: its packages now live (for good) on archive.debian.org
    printf 'deb http://archive.debian.org/debian bullseye main\ndeb http://archive.debian.org/debian bullseye-updates main\ndeb http://archive.debian.org/debian-security bullseye-security main\n' > /etc/apt/sources.list
    dpkg --add-architecture arm64
    apt-get update -q
    DEBIAN_FRONTEND=noninteractive apt-get install -yq --no-install-recommends ca-certificates curl git gcc libc6-dev \
        gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libc6-dev-arm64-cross pkg-config libasound2-dev:arm64 libudev-dev:arm64
    ;;
build)
    rustup target add $t
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
        CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
        AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar \
        PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig:/usr/share/pkgconfig
    # HST_FEATURES= (empty) builds without `ship`, keeping the developer HST_* variables for a device session (M155b)
    cargo build --release -p hst --features "${HST_FEATURES-ship}" --target $t
    out=dist/linux-aarch64/OpenTennis
    rm -rf dist/linux-aarch64 && mkdir -p $out  # the tarball is the whole folder: nothing stale in it
    aarch64-linux-gnu-strip -o $out/opentennis "${CARGO_TARGET_DIR:-target}/$t/release/hst"
    cp crates/hst/linux/OpenTennis.sh $out/
    cp crates/hst/icon/hst.png $out/opentennis.png
    printf '[Desktop Entry]\nType=Application\nName=OpenTennis\nExec=opentennis\nIcon=opentennis\nStartupWMClass=opentennis\nCategories=Game;\n' > $out/opentennis.desktop
    need=$(aarch64-linux-gnu-objdump -T $out/opentennis | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)
    echo "$out/opentennis needs $need (built against GLIBC_$glibc)"
    if [ "$(printf '%s\n' "${need#GLIBC_}" $glibc | sort -V | tail -1)" != $glibc ]; then
        echo "$out/opentennis needs $need, newer than GLIBC_$glibc" >&2
        exit 1
    fi
    ;;
*) echo "usage: tools/linux-aarch64.sh setup|build" >&2; exit 2 ;;
esac

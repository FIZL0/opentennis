#!/bin/bash
# tools/lavapipe.sh x86_64|aarch64 DIR — B708: Mesa's software Vulkan driver (lavapipe: llvmpipe behind Vulkan) for the
# Linux builds, so Settings → Display → Rendering → Software has something to draw with on a system that ships no
# software Vulkan (SteamOS, most distros: it's an extra package there). Puts into DIR/vulkan/:
#   libvulkan_lvp.so   the driver, LLVM linked in (no libLLVM to carry), built for glibc 2.31 like the game
#   lvp_icd.json       its Vulkan ICD manifest (library_path relative to it); the game adds it with VK_ADD_DRIVER_FILES
#                      at start, only when Software is picked (crates/hst-base/src/render_mode.rs)
#   LICENSE-mesa.txt   Mesa's licence (MIT), and LLVM's (Apache 2.0 with LLVM exception)
# tools/build.sh linux / linux-aarch64 call it. The build (~5 min, once per version) is cached in ~/.cache/hst-lavapipe/<arch>;
# the sources are pinned by checksum. x86_64 builds in Valve's sniper SDK container (the game's own, tools/build.sh);
# aarch64 in an arm64 Debian 11 container under QEMU (docker's binfmt: `docker run --privileged --rm tonistiigi/binfmt
# --install arm64` once), slow (an hour) but once.
set -e
cd "$(dirname "$0")/.."
arch=$1 dir=$2
[ -n "$dir" ] || { echo "usage: tools/lavapipe.sh x86_64|aarch64 DIR" >&2; exit 2; }
mesa=24.0.9
cache=~/.cache/hst-lavapipe
src=$cache/src
out=$cache/$arch-$mesa
fetch() {  # name url sha256
    [ -f "$src/$1" ] && echo "$3  $src/$1" | sha256sum -c --quiet && return
    curl -sSfL -o "$src/$1" "$2"
    echo "$3  $src/$1" | sha256sum -c --quiet || { rm -f "$src/$1"; echo "$1: checksum mismatch" >&2; exit 1; }
}
if [ ! -f "$out/libvulkan_lvp.so" ]; then
    command -v docker >/dev/null && docker info >/dev/null 2>&1 || { echo "lavapipe needs docker (see tools/build.sh)" >&2; exit 1; }
    mkdir -p "$src" "$out"
    fetch mesa.tar.xz https://archive.mesa3d.org/mesa-$mesa.tar.xz 51aa686ca4060e38711a9e8f60c8f1efaa516baf411946ed7f2c265cd582ca4c
    fetch wayland-protocols.tar.xz https://gitlab.freedesktop.org/wayland/wayland-protocols/-/releases/1.33/downloads/wayland-protocols-1.33.tar.xz 94f0c50b090d6e61a03f62048467b19abbe851be4e11ae7b36f65f8b98c3963a
    fetch libdrm.tar.xz https://dri.freedesktop.org/libdrm/libdrm-2.4.120.tar.xz 3bf55363f76c7250946441ab51d3a6cc0ae518055c0ff017324ab76cdefb327a
    fetch libxshmfence.tar.xz https://www.x.org/releases/individual/lib/libxshmfence-1.3.2.tar.xz 870df257bc40b126d91b5a8f1da6ca8a524555268c50b59c0acd1a27f361606f
    fetch meson.whl https://files.pythonhosted.org/packages/py3/m/meson/meson-1.4.1-py3-none-any.whl d5acc3abae2dad3c70ddcbd10acac92b78b144d34d43f40f5b8ac31dfd8a826a
    fetch mako.whl https://files.pythonhosted.org/packages/py3/M/Mako/Mako-1.3.5-py3-none-any.whl 260f1dbc3a519453a9c856dedfe4beb4e50bd5a26d96386cb6c80856556bb91a
    fetch markupsafe.tar.gz https://files.pythonhosted.org/packages/source/M/MarkupSafe/MarkupSafe-2.1.5.tar.gz d283d37a890ba4c1ae73ffadf8046435c76e7bc2247bbb63c00bd1a709c6544b
    case $arch in
        x86_64) img=registry.gitlab.steamos.cloud/steamrt/sniper/sdk:latest plat=linux/amd64 setup= ;;
        aarch64) img=arm64v8/debian:bullseye plat=linux/arm64
            setup='printf "deb http://archive.debian.org/debian bullseye main\ndeb http://archive.debian.org/debian bullseye-updates main\ndeb http://archive.debian.org/debian-security bullseye-security main\n" > /etc/apt/sources.list
                apt-get update -q && DEBIAN_FRONTEND=noninteractive apt-get install -yq --no-install-recommends gcc g++ pkg-config \
                    python3 python3-distutils ninja-build bison flex llvm-11-dev libclang-common-11-dev libz-dev libexpat1-dev libwayland-dev libx11-dev libx11-xcb-dev \
                    libxcb1-dev libxcb-dri3-dev libxcb-dri2-0-dev libxcb-present-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-sync-dev \
                    libxcb-xfixes0-dev libxcb-keysyms1-dev libxrandr-dev libudev-dev libelf-dev libwayland-bin glslang-tools make xz-utils >/dev/null' ;;
        *) echo "usage: tools/lavapipe.sh x86_64|aarch64 DIR" >&2; exit 2 ;;
    esac
    # as root inside (the arm64 image installs its tools), then the result handed back to this user
    docker run --rm --platform $plat -v "$src:/dl:ro" -v "$out:/out" -e SETUP="$setup" -e OWNER="$(id -u):$(id -g)" $img bash -ec '
        trap "tail -20 /b/deps.log 2>/dev/null; echo \"lavapipe build failed: \$BASH_COMMAND\" >&2" ERR
        eval "$SETUP"
        mkdir -p /b/py && cd /b
        python3 -c "import zipfile; [zipfile.ZipFile(\"/dl/\"+w).extractall(\"py\") for w in (\"meson.whl\", \"mako.whl\")]"
        tar -xzf /dl/markupsafe.tar.gz && cp -r MarkupSafe-*/src/markupsafe py/
        export PYTHONPATH=/b/py
        tar -xJf /dl/wayland-protocols.tar.xz && (cd wayland-protocols-* && python3 -m mesonbuild.mesonmain setup --prefix=/b/wp -Dtests=false build >>/b/deps.log 2>&1 && ninja -C build install >>/b/deps.log 2>&1)
        # libdrm newer than Debian 11 has, linked in (static)
        tar -xJf /dl/libdrm.tar.xz && (cd libdrm-* && python3 -m mesonbuild.mesonmain setup --prefix=/b/wp --libdir=lib -Ddefault_library=static \
            -Dintel=disabled -Dradeon=disabled -Damdgpu=disabled -Dnouveau=disabled -Dvmwgfx=disabled -Dfreedreno=disabled \
            -Dvc4=disabled -Detnaviv=disabled -Dexynos=disabled -Domap=disabled -Dtegra=disabled -Dcairo-tests=disabled \
            -Dman-pages=disabled -Dvalgrind=disabled -Dtests=false -Dc_args=-fPIC build >>/b/deps.log 2>&1 && ninja -C build install >>/b/deps.log 2>&1)
        tar -xJf /dl/libxshmfence.tar.xz && (cd libxshmfence-* && ./configure -q --prefix=/b/wp --libdir=/b/wp/lib --disable-shared --with-pic >>/b/deps.log 2>&1 && make -s install >>/b/deps.log 2>&1)
        export PKG_CONFIG_PATH=/b/wp/share/pkgconfig:/b/wp/lib/pkgconfig
        tar -xJf /dl/mesa.tar.xz && cd mesa-*
        # lavapipe alone, X11 + Wayland presentation, LLVM static (one .so to ship), no GL
        python3 -m mesonbuild.mesonmain setup build --buildtype=release -Db_ndebug=true -Dstrip=true \
            -Dplatforms=x11,wayland -Dvulkan-drivers=swrast -Dgallium-drivers=swrast -Dllvm=enabled -Dshared-llvm=disabled \
            -Dopengl=false -Dgles1=disabled -Dgles2=disabled -Degl=disabled -Dglx=disabled -Dgbm=disabled \
            -Dvideo-codecs= -Dzstd=disabled -Dlibunwind=disabled -Dvalgrind=disabled -Dxmlconfig=disabled \
            -Dbuild-tests=false -Dvulkan-layers= -Dtools= >/b/configure.log 2>&1 || { tail -40 /b/configure.log; exit 1; }
        ninja -C build src/gallium/targets/lavapipe/libvulkan_lvp.so >/b/build.log 2>&1 || { tail -40 /b/build.log; exit 1; }
        cp build/src/gallium/targets/lavapipe/libvulkan_lvp.so /out/
        cp docs/license.rst /out/LICENSE-mesa.txt
        cp /usr/share/doc/libllvm11/copyright /out/LICENSE-llvm.txt
        chown -R "$OWNER" /out'
fi
need=$(objdump -T "$out/libvulkan_lvp.so" 2>/dev/null | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)
mkdir -p "$dir/vulkan"
cp "$out/libvulkan_lvp.so" "$out"/LICENSE-*.txt "$dir/vulkan/"
cat >"$dir/vulkan/lvp_icd.json" <<EOF
{
    "file_format_version": "1.0.0",
    "ICD": {
        "library_path": "./libvulkan_lvp.so",
        "api_version": "1.3.274"
    }
}
EOF
echo "$dir/vulkan: lavapipe (Mesa $mesa${need:+, needs $need})"

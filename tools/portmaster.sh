#!/bin/bash
# tools/portmaster.sh — M155a: the ARM Linux build (dist/linux-aarch64/OpenTennis/opentennis, tools/build.sh
# linux-aarch64 runs this after it) as a PortMaster port (https://portmaster.games/packaging.html):
#   dist/linux-aarch64/opentennis/     the port's folder as PortMaster's repository takes it: port.json, README.md,
#                                      screenshot.png, gameinfo.xml, OpenTennis.sh, opentennis/ (the game)
#   dist/linux-aarch64/opentennis.zip  what a player installs (PortMaster's autoinstall/, or unzip into ports/ with
#                                      OpenTennis.sh moved to the Ports menu's folder): OpenTennis.sh at the top,
#                                      opentennis/ holding the game, licenses/ and the port's metadata
# The files come from crates/hst/linux/portmaster/ (min_glibc filled in from the binary), the screenshot from
# docs/screenshots/portmaster.png (our renderer, a 4:3 match). No game data goes in.
set -e
cd "$(dirname "$0")/.."
src=crates/hst/linux/portmaster
bin=dist/linux-aarch64/OpenTennis/opentennis
[ -f "$bin" ] || { echo "no $bin: tools/build.sh linux-aarch64 first" >&2; exit 1; }
out=dist/linux-aarch64/opentennis
rm -rf "$out" dist/linux-aarch64/opentennis.zip dist/linux-aarch64/zip
mkdir -p "$out/opentennis/licenses" "$out/opentennis/libs.aarch64"

# the newest glibc symbol the game links (PortMaster won't offer the port to an older system)
glibc=$(readelf --dyn-syms -W "$bin" | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)
glibc=${glibc#GLIBC_}
[ -n "$glibc" ] || { echo "no GLIBC_ symbols in $bin?" >&2; exit 1; }
sed "s/@MIN_GLIBC@/$glibc/" $src/port.json >"$out/port.json"
cp $src/README.md $src/gameinfo.xml "$out/"
cp docs/screenshots/portmaster.png "$out/screenshot.png"
install -m 755 $src/OpenTennis.sh "$out/OpenTennis.sh"
install -m 755 "$bin" "$out/opentennis/opentennis.aarch64"
# no bundled .so: libs.aarch64 stays empty (the launcher puts it on LD_LIBRARY_PATH, as the template does)
cp LICENSE-MIT LICENSE-APACHE "$out/opentennis/licenses/"
cp crates/hst/assets/fonts/OFL.txt "$out/opentennis/licenses/OFL-MPLUSRounded1c.txt"
cp CREDITS.md "$out/opentennis/licenses/CREDITS.md"

# the install zip: PortMaster reads port.json (and gameinfo.xml, the screenshot) from inside the port's folder
z=dist/linux-aarch64/zip
mkdir -p "$z"
cp -a "$out/OpenTennis.sh" "$z/"
cp -a "$out/opentennis" "$z/"
cp -a "$out/port.json" "$out/README.md" "$out/gameinfo.xml" "$out/screenshot.png" "$z/opentennis/"
(cd "$z" && zip -qrX ../opentennis.zip OpenTennis.sh opentennis)
rm -rf "$z"
echo "dist/linux-aarch64/opentennis.zip (min_glibc $glibc)"

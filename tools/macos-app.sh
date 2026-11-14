#!/usr/bin/env bash
# M162: OpenTennis.app for Apple Silicon Macs, zipped: release build (tools/macos-build.sh), stripped, Info.plist +
# icon, ad-hoc signed with rcodesign (Apple Silicon runs nothing unsigned; no Apple account, so no notarization: the
# player opens it once with right-click → Open, or System Settings → Privacy & Security → Open Anyway).
# Like Linux, the first start asks for the ISO (disc.rs) and keeps the game's files in ~/Library/Application Support/hst.
# Usage: tools/macos-app.sh [OUT.zip]   (default target/opentennis-macos-arm64.zip)
# Needs (once): the SDK (tools/macos-build.sh), cargo install apple-codesign (rcodesign), ImageMagick (the icon).
set -euo pipefail
cd "$(dirname "$0")/.."
out=$(realpath -m "${1:-target/opentennis-macos-arm64.zip}")
command -v rcodesign >/dev/null || { echo "needs (once): cargo install apple-codesign   (rcodesign, the ad-hoc signer)" >&2; exit 1; }
tools/macos-build.sh build --release -p hst --features ship
stage=target/aarch64-apple-darwin/app
app=$stage/OpenTennis.app
rm -rf "$stage"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
llvm-strip -o "$app/Contents/MacOS/opentennis" target/aarch64-apple-darwin/release/hst
# the icon: an .icns of PNG entries (ic07 128, ic08 256, ic09 512, ic10 1024 px)
icons=$stage/icons && mkdir -p "$icons"
for px in 128 256 512 1024; do magick web/icon-512.png -resize ${px}x$px "$icons/$px.png"; done
python3 -I - "$icons" "$app/Contents/Resources/AppIcon.icns" <<'PY'
import struct, sys
d, out = sys.argv[1:]
body = b"".join(t + struct.pack(">I", 8 + len(png)) + png for t, png in
                ((t.encode(), open(f"{d}/{px}.png", "rb").read()) for t, px in (("ic07", 128), ("ic08", 256), ("ic09", 512), ("ic10", 1024))))
open(out, "wb").write(b"icns" + struct.pack(">I", 8 + len(body)) + body)
PY
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)  # [workspace.package]
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key><string>opentennis</string>
	<key>CFBundleIdentifier</key><string>io.github.fizl0.hst</string>
	<key>CFBundleName</key><string>OpenTennis</string>
	<key>CFBundleDisplayName</key><string>OpenTennis</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleIconFile</key><string>AppIcon</string>
	<key>CFBundleShortVersionString</key><string>$version</string>
	<key>CFBundleVersion</key><string>$(git rev-list --count HEAD)</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleSupportedPlatforms</key><array><string>MacOSX</string></array>
	<key>LSMinimumSystemVersion</key><string>11.0</string>
	<key>LSArchitecturePriority</key><array><string>arm64</string></array>
	<key>LSApplicationCategoryType</key><string>public.app-category.sports-games</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSSupportsAutomaticGraphicsSwitching</key><true/>
	<key>NSLocalNetworkUsageDescription</key><string>Finds and joins matches on your local network.</string>
</dict>
</plist>
PLIST
rcodesign sign "$app" >/dev/null  # no certificate given: an ad-hoc signature (binds Info.plist and the binary)
rm -f "$out"
(cd "$stage" && zip -qry -X "$out" OpenTennis.app)
echo "$out ($(du -h "$out" | cut -f1))"

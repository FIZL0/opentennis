#!/bin/sh
# M152c: the guest page as an adhoc host serves it. A native host (`hst net host`, `HST_ADHOC=<disc>`: the page, the
# disc as a pack and the mods over http, `/rtc` on the same port) and a headless-Chromium guest with an empty profile
# that opens only http://<host ip>:<port>/ (tools/web_adhoc.mjs, GUEST_URL): it loads the disc and mods from the host
# into memory and joins the lobby by itself. Every seat a bot on both ends. Needs target/release/hst and web/pkg
# (tools/web.sh). Usage: tools/web_guest.sh DISC OUT [seconds of match]. Host log: OUT/host.txt.
set -e
cd "$(dirname "$0")/.."
out=$2 secs=${3:-60}
mkdir -p "$out"
out=$(realpath "$out")
# a link in OUT, so the pack the host makes from an ISO lands beside it (OUT/cache), not beside the real disc
ln -sfn "$(realpath "$1")" "$out/disc"
disc=$out/disc
ip=$(ip -4 route get 1.1.1.1 | sed -n 's/.* src \([0-9.]*\).*/\1/p')
export WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-1}
# the host runs in OUT: a `mods` there is what it serves (none: the guest picks disc characters; the CLI host doesn't
# start a match on a guest's mod pick yet)
bin=$PWD/target/release/hst web=$PWD/web
(cd "$out" && HST_BG=1 HST_AUTOPLAY=1 HST_ADHOC="$disc" HST_WEB="$web" RUST_LOG=info exec timeout $((secs + 300)) "$bin" net host --relay off --games 1 --play "$disc") >"$out/host.txt" 2>&1 &
host=$!
trap 'kill $host 2>/dev/null' EXIT
port=
for _ in $(seq 60); do
  port=$(sed -n 's/.*adhoc: serving at .*http:[^"]*:\([0-9]*\)".*/\1/p' "$out/host.txt" | head -n 1)
  [ -n "$port" ] && break
  sleep 1
done
[ -n "$port" ] || { echo "no adhoc server"; tail "$out/host.txt"; exit 1; }
echo "host: http://$ip:$port/"
rm -rf "$out/profile"
# the GPU through ANGLE/Vulkan: on SwiftShader the guest is still loading the court a minute in
GUEST_URL="http://$ip:$port/?webgl2&autoplay" CHROME_FLAGS="${CHROME_FLAGS:---use-angle=vulkan --enable-features=Vulkan}" node tools/web_adhoc.mjs - "$out" "$ip:$port" "$secs"
grep -E "hash|Joined|Start|LAN|adhoc|panicked" "$out/host.txt" | tail -20

#!/bin/sh
# M152b: a native host (`hst net host`, signalling on HST_RTC_SIGNAL) and a headless-Chromium guest that joins it by
# its address (tools/web_adhoc.mjs), every seat a bot on both ends. Needs target/release/hst and web/pkg built
# (tools/web.sh). Usage: tools/web_adhoc.sh ISO|- OUT [seconds of match]. Host log: OUT/host.txt.
set -e
cd "$(dirname "$0")/.."
iso=$1 out=$2 secs=${3:-60}
port=${RTC_PORT:-8791}
mkdir -p "$out"
# the address a guest on the LAN would use: this machine's on its default route
ip=$(ip -4 route get 1.1.1.1 | sed -n 's/.* src \([0-9.]*\).*/\1/p')
python3 -m http.server -d web "${WEB_PORT:-8766}" >/dev/null 2>&1 &
web=$!
export WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-1}
# the match window: titled "(shot)", silent, under the user's (HST_BG)
HST_BG=1 HST_AUTOPLAY=1 HST_RTC_SIGNAL=$port RUST_LOG=info timeout $((secs + 240)) target/release/hst net host --relay off --games 1 --play "${HST_ISO:-Hot Shots Tennis (USA).iso}" >"$out/host.txt" 2>&1 &
host=$!
trap 'kill $web $host 2>/dev/null' EXIT
sleep 3
node tools/web_adhoc.mjs "$iso" "$out" "$ip:$port" "$secs"
grep -E "hash|Joined|Start|LAN|panicked" "$out/host.txt" | tail -20

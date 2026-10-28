#!/bin/sh
# M152g: a native app with no disc joins an adhoc host. A native host (`hst net host`, `HST_ADHOC=<disc>`) and a native
# guest with an empty data folder (XDG_DATA_HOME=OUT/guest-data) and no disc: its no-disc page finds the host
# (`HST_JOIN=<ip:port>` picks it, as a click on the found host would), gets the disc, restarts into memory and joins the
# lobby; `HST_KEYS` presses through the seat and the picks, then the lobby's Ready; the match starts. Every seat a bot on both ends. Needs target/release/hst.
# Usage: tools/native_guest.sh DISC OUT [seconds]. Logs: OUT/host.txt, OUT/guest.txt; OUT/guest-data must end empty
# of disc files.
set -e
cd "$(dirname "$0")/.."
out=$2 secs=${3:-90}
mkdir -p "$out"
out=$(realpath "$out")
ln -sfn "$(realpath "$1")" "$out/disc"
disc=$out/disc
ip=$(ip -4 route get 1.1.1.1 | sed -n 's/.* src \([0-9.]*\).*/\1/p')
export WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-1}
bin=$PWD/target/release/hst
(cd "$out" && HST_BG=1 HST_AUTOPLAY=1 HST_ADHOC="$disc" RUST_LOG=info exec timeout $((secs + 120)) "$bin" net host --relay off --games 1 --play "$disc") >"$out/host.txt" 2>&1 &
host=$!
trap 'kill $host $guest 2>/dev/null' EXIT
port=
for _ in $(seq 60); do
  port=$(sed -n 's/.*adhoc: serving at .*http:[^"]*:\([0-9]*\)".*/\1/p' "$out/host.txt" | head -n 1)
  [ -n "$port" ] && break
  sleep 1
done
[ -n "$port" ] || port=$(sed -n 's/.*adhoc: serving at .*"http:\/\/[0-9.]*".*/80/p' "$out/host.txt" | head -n 1)
[ -n "$port" ] || { echo "no adhoc server"; tail "$out/host.txt"; exit 1; }
echo "host: $ip:$port"
rm -rf "$out/guest-data"
mkdir -p "$out/guest-data"
(cd "$out" && XDG_DATA_HOME="$out/guest-data" HST_BG=1 HST_AUTOPLAY=1 HST_JOIN="$ip:$port" RUST_LOG=info \
  HST_KEYS="${GUEST_KEYS:-5:Enter 8:Enter 10:ArrowDown 11:ArrowDown 12:ArrowDown 13:ArrowDown 14:Enter}" \
  exec timeout "$secs" "$bin") >"$out/guest.txt" 2>&1 &
guest=$!
wait $guest || true
echo "guest data left:"; find "$out/guest-data" -type f | head
grep -E "adhoc guest|Joining|Joined|hash|panicked|error" "$out/guest.txt" | tail -15
grep -E "hash|Joined|Start|adhoc|panicked" "$out/host.txt" | cut -c1-200 | tail -10

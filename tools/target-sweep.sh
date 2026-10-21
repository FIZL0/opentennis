#!/usr/bin/env bash
# Delete build outputs nobody has read in DAYS days (default 3) from cargo target dirs: every rebuild leaves the old
# hashed copies behind (a slot's target/ grew to ~110 GB, half of it stale). A file cargo still wants is simply
# rebuilt (sccache makes that quick). Needs atime (relatime is enough: it updates at least once a day).
# tools/target-sweep.sh [DAYS] [DIR…]   (no DIR: this checkout's target*/; runner.toml's on_exit runs it per slot)
# ponytail: atime heuristic, not cargo's own dependency graph; switch to cargo-sweep if a needed file ever goes missing
days=${1:-3}; shift
cd "$(dirname "$(realpath "$0")")/.." || exit 1
[ $# = 0 ] && set -- target*/
for t in "$@"; do
  [ -f "$t/CACHEDIR.TAG" ] || { echo "skip $t: not a cargo target dir"; continue; }
  before=$(du -s "$t" | cut -f1)
  find "$t" -type f -atime +"$days" \( -path '*/deps/*' -o -path '*/incremental/*' -o -path '*/build/*' -o -path '*/.fingerprint/*' \) -delete
  find "$t" -mindepth 2 -type d -empty -delete
  echo "$t: freed $(( (before - $(du -s "$t" | cut -f1)) / 1048576 )) GB"
done

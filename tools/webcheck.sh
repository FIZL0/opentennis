#!/bin/sh
# tools/webcheck.sh — B618: the browser build (tools/web.sh's wasm32 target, `ship` as released) type-checks with no
# warning in our code, so something that only breaks or warns in the web build (a cfg'd-out user, a field the web
# side names) fails a task's done.sh and release.sh before the release's builds start, not in pages.yml after.
# Prints nothing but the problems; exit 1 on any. Shares web.sh's target-wasm/.
cd "$(dirname "$0")/.." || exit 1
out=$(CARGO_TARGET_DIR=$PWD/target-wasm cargo check -q --locked -p hst --target wasm32-unknown-unknown --release --features ship --message-format=short 2>&1)
rc=$?
bad=$(printf '%s\n' "$out" | grep -E '^(crates|tools|third_party)/.*(error|warning)|^error' | grep -v 'could not compile')
[ $rc = 0 ] && [ -z "$bad" ] && exit 0
printf '%s\n' "${bad:-$out}" | head -30
echo "WEB CHECK FAILED (CARGO_TARGET_DIR=target-wasm cargo check -p hst --target wasm32-unknown-unknown --release --features ship)"
exit 1

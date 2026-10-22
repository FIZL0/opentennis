#!/usr/bin/env bash
# Quiet `cargo test` for agents: tools/check.sh [cargo test args, e.g. -p hst-sim --test flights].
# Passing: one line. Build error: the error lines only. Failing tests: their panic messages only.
# Every line cut to 300 chars, at most 60 lines. Full output: context/notes/check.log.
cd "$(dirname "$(realpath "$0")")/.." || exit 1
log=context/notes/check.log
mkdir -p context/notes
# HST_CHECK_SLOTS test runs at once across main and every worktree (default 3: 30 GB RAM fits three Bevy links; more swaps), each capped to its share of the
# cores: five uncapped at once (each using every core) took ~30 min apiece, one at a time left cores idle
# during links and long single-threaded tests
n=${HST_CHECK_SLOTS:-3} jobs=$(( $(nproc) / ${HST_CHECK_SLOTS:-3} )); [ "$jobs" -ge 1 ] || jobs=1
lock="$(git rev-parse --git-common-dir)/check.lock"
# T7: the wait for a slot is bounded (HST_CHECK_WAIT seconds, default 3600) and gives up at once when sccache holds
# every slot (a server started by a cargo that inherited fd 9, before the 9>&- below: it never lets go)
grab() { for i in $(seq 0 $((n - 1))); do exec 9>"$lock.$i"; flock -n 9 && return 0; done; exec 9>&-; return 1; }
holders() { lslocks -n -o COMMAND,PID,PATH 2>/dev/null | awk -v l="$lock." 'index($3, l) == 1 { print $1, $2, $3 }'; }
waited=0
until grab; do
  h=$(holders)
  if [ -n "$h" ] && [ "$(awk '$1 ~ /^sccache/ { print $3 }' <<<"$h" | sort -u | wc -l)" -ge "$n" ]; then
    echo "CHECK FAILED: sccache holds every test slot lock (it inherited one from an old check.sh and never exits):"
    echo "$h"; echo "fix: sccache --stop-server; sccache --start-server (from a shell holding no lock), then rerun"; exit 1
  fi
  if [ "$waited" -ge "${HST_CHECK_WAIT:-3600}" ]; then
    echo "CHECK FAILED: no free test slot after ${waited}s (HST_CHECK_WAIT); holders (command, pid, lock):"
    echo "${h:-(lslocks shows none)}"; exit 1
  fi
  [ "$waited" = 0 ] && echo "waiting for a test slot (all $n busy, up to ${HST_CHECK_WAIT:-3600}s)" >&2
  sleep 5; waited=$((waited + 5))
done
export RUST_TEST_THREADS=${RUST_TEST_THREADS:-$jobs}
# cargo never deletes stale artifacts (target/ reached 230 GB, 1.2 TB across the slots): any target dir of this
# checkout (target/, target-wasm/, ...) past HST_TARGET_CAP_GB is started over; sccache makes the rebuild cheap.
# cargo clean waits on cargo's own build lock, so it never pulls files from under a running build.
# ponytail: whole-dir wipe, cargo-sweep by age if the rebuilds get annoying
# Incremental caches (about half of it, only our own crates) go first: a full wipe rebuilds all of Bevy, which
# misses sccache in every worktree (the patched path crates in third_party/ give Bevy a per-checkout hash).
over() { [ -d "$1" ] && [ "$(du -s --block-size=1G "$1" | cut -f1)" -gt "${HST_TARGET_CAP_GB:-120}" ]; }
for t in target target-*; do
  over "$t" && { echo "$t over ${HST_TARGET_CAP_GB:-120} GB, dropping incremental caches"
    find "$t" -maxdepth 3 -type d -name incremental -prune -exec rm -rf {} +; }
  over "$t" && { echo "$t still over, cleaning it"; cargo clean -q --target-dir "$t" 9>&-; }
done
# 9>&-: cargo mustn't inherit the lock, or the sccache server it starts holds it forever
# The netplay match tests (two-to-four Apps playing thousands of ticks, minutes each) are skipped unless asked for:
# HST_NETPLAY=1, or a test filter naming netplay/rollback/synctest. Touch netplay, net or the snapshot → run them.
args=("$@")
if [ -z "$HST_NETPLAY" ] && [[ "$*" != *netplay* && "$*" != *rollback* && "$*" != *synctest* ]]; then
  [[ " $* " == *" -- "* ]] || args+=(--)
  args+=(--skip play::netplay::tests::matches)
fi
# T5: a test whose disc or context/ recording is missing fails instead of passing without checking anything
# (hst_data::fixture); HST_REQUIRE_DISC=0 for a checkout without them
export HST_REQUIRE_DISC=${HST_REQUIRE_DISC-1}
# every recording a test reads, for tools/context_prune.py (the main checkout's context/, which slots link to)
# nice: builds yield to the desktop and PCSX2
nice -n 10 cargo test -j "$jobs" -q --message-format=short "${args[@]}" >"$log" 2>&1 9>&- && {
  echo "TESTS PASSED: $(grep -ao '[0-9]* passed' "$log" | awk '{s+=$1} END{print s+0}') tests"; exit 0; }
if grep -q '^test result: FAILED' "$log"; then
  echo "TESTS FAILED (this is a test result, the build is fine):"
  awk '/^failures:$/{f++; next} f==1 && !/^note: run with/ && NF' "$log"
else
  echo "BUILD FAILED (treat this as a build failure, no tests ran):"
  grep -E '(^|: )error(\[E[0-9]+\])?:' "$log"
fi | cut -c1-300 | head -60
echo "(full log: $log)"
exit 1

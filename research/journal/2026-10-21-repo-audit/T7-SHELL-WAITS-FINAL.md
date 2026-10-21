# T7 — shell script robustness (from the audit's *Shell* finding)

single.sh / parallel.sh / agent-stop.sh are gone (agent-runner), so their parts were skipped.

## Bounded waits
- `tools/check.sh`: the slot wait is a loop with a cap, `HST_CHECK_WAIT` seconds (default 3600). On timeout it prints
  `CHECK FAILED: no free test slot …` plus the holders (`lslocks`: command, pid, lock) and exits 1. If `sccache`
  holds every slot's lock (the old fd-9 inheritance bug: it never lets go) it fails at once with the fix
  (`sccache --stop-server; sccache --start-server`). When it starts waiting it says so once on stderr.
- `tools/pcsx2.sh`: `flock -w $HST_PCSX2_WAIT` (default 1800 s, more than the 1200 s hold cap); on timeout nothing runs,
  exit 75, with a message naming the lock.
- Tested on scratch lock files: a held lock times out (exit 1 / 75) with the message, a fake `sccache` holder fails
  check.sh straight away, a free lock goes through.

## set -e / guards (13 scripts lacked set -e now)
- `set -e` added where every command's failure should stop the script: `tools/since.sh`, `research/b211_snap.sh`.
- Not safe for `set -e` (they read exit codes of grep/git grep/wait, or must keep looping), so explicit guards instead:
  `cd … || exit 1` in check.sh, done.sh, audit.sh, todo.sh, asset_sweep.sh, select_stress.sh; `mkdir … || exit 1` in
  asset_sweep.sh, select_stress.sh; vram_watch.sh exits with a message when no card exposes VRAM use (it would have
  watched nothing and never killed a leaking run).
- Left as they are: `research/fn.sh`, `xr.sh`, `dis.sh` (one command / `exec`; their exit code is already the
  command's).

## Not verified / not 1:1
none (tooling only; nothing gameplay-related).

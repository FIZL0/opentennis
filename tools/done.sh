#!/usr/bin/env bash
# Wrap a task up in one call (no subagent): tools/done.sh <ID> "<commit message>" [--blocked "<why + what unblocks>" | --spec] [-- <paths>]
# Runs tools/audit.sh (no retail addresses), tools/patches.py (third_party/ documented), the tests and the sim's float lint; passing: ticks <ID> (moves it to plan/DONE.md, its research/<id>_* probes into its journal) or, with --blocked, marks it [~]; then commits
# everything (in a worktree) or only plan/ + <paths> (main checkout, where other sessions' edits sit). Failing: nothing is ticked or committed.
# --spec (clean-room ports, PLAN.md): the session wrote or questioned the task's SPEC.md; audit, commit as "<ID> spec: …"
# without ticking, so the task stays open for its next phase (the runner may start it again the same night).
cd "$(dirname "$(realpath "$0")")/.." || exit 1
paths=(); for ((i = 1; i <= $#; i++)); do [ "${!i}" = -- ] && { paths=("${@:i+1}"); set -- "${@:1:i-1}"; break; }; done
[ $# = 2 ] || { [ $# = 4 ] && [ "$3" = --blocked ]; } || { [ $# = 3 ] && [ "$3" = --spec ]; } || { sed -n 2,6p "$0"; exit 2; }
main=; [ "$(git rev-parse --git-dir)" = "$(git rev-parse --git-common-dir)" ] && main=1
[ "$main" ] && [ ${#paths[@]} = 0 ] && { echo "main checkout: name the paths you changed: tools/done.sh <ID> \"<msg>\" -- <paths> (git add -A would commit other sessions' edits)"; exit 2; }
if [ "$3" = --spec ]; then
  tools/audit.sh || exit 1
  if [ "$main" ]; then git add -- "${paths[@]}" && git commit -qm "$1 spec: $2" -- "${paths[@]}"
  else git add -A && git commit -qm "$1 spec: $2"; fi && rm -f .cleanroom && git log -1 --oneline
  exit
fi
tools/audit.sh && tools/patches.py && tools/check.sh || exit 1
# T9: CI's clippy (ci.yml), warnings are errors, so a task can't land a warning that stops the beta. It covers the
# sim's #![deny(clippy::float_arithmetic)] modules too (C2: float math through ps2::F / vu0::Vf)
out=$(cargo clippy -q --locked --workspace --all-targets --message-format=short -- -D warnings 2>&1) || {
  grep -E '^(crates|tools|third_party)/.*(error|warning)|^error' <<<"$out" | grep -v 'could not compile' | head -30
  grep -q float_arithmetic <<<"$out" && echo "plain f32 arithmetic in a converted sim module: use ps2::F / vu0::Vf"
  echo "CLIPPY FAILED (same as CI's: cargo clippy --workspace --all-targets -- -D warnings)"; exit 1; }
export HST_CTX_NOCOMMIT=1  # the tick goes into the commit below
if [ $# = 4 ]; then tools/ctx.py block "$1" "$4"; else tools/ctx.py tick "$1"; fi || exit 1
# T6: a done task's probes (research/<id>_*.py) move into its journal's scripts/ (the main checkout commits only <paths>)
[ $# = 2 ] && [ -z "$main" ] && { tools/probes.py "$1" || exit 1; }
# R10: a ticked task's own journal ends -FINAL: its newest state file, if -READY/-PART, is renamed (AGENT.md, Research)
if [ $# = 2 ]; then
  while read -r d; do
    f=$(ls "$d" | grep -E -- '-(FINAL|PART|READY)\.md$' | sort -V | tail -1)
    [[ $f =~ -(READY|PART)\.md$ ]] && git mv "$d/$f" "$d/${f%-*}-FINAL.md" && echo "journal: $d/${f%-*}-FINAL.md"
    paths+=("$d")
  done < <(find research/journal -mindepth 1 -maxdepth 1 -type d -regextype posix-extended -iregex ".*/[0-9]{4}-[0-9]{2}-[0-9]{2}-$1(-.*)?")
fi
if [ "$main" ]; then git add -- "${paths[@]}" && git commit -qm "$2" -- plan/TODO.md plan/DONE.md "${paths[@]}"
else git add -A && git commit -qm "$2"; fi && rm -f .cleanroom && git log -1 --oneline

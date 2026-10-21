#!/usr/bin/env bash
# Remove leftover ../opentennis-<slug> worktrees (concept branches, finished ad-hoc work): runner.toml's on_exit runs it
# after every slot session. A worktree goes only when nothing is lost and nobody is in it:
#   - no process has its cwd inside it (a live session, a build);
#   - no live Claude session (but the one closing, whose hook this is) names it in its transcript: a session runs
#     its commands from its own cwd, so between commands only its transcript says it works in the folder;
#   - its HEAD is on main or on origin (a concept/* branch is pushed first);
#   - `git worktree remove` (no --force) agrees: no uncommitted or untracked files.
# The branch itself is kept; only the folder goes. `-n` lists what it would remove; `--hook` is the SessionEnd hook.
cd "$(dirname "$(realpath "$0")")/.." || exit 1
dry=; hook=; for a; do [ "$a" = -n ] && dry=1; [ "$a" = --hook ] && hook=1; done
main=$(git rev-parse --show-toplevel)
# transcripts of live Claude sessions; with --hook, not this script's own ancestors (the session closing)
anc=" "; p=$$; [ "$hook" ] && while [ "${p:-1}" -gt 1 ]; do anc+="$p "; p=$(ps -o ppid= -p "$p" | tr -d ' '); done
live=(); for f in ~/.claude/sessions/*.json; do
  pid=$(basename "$f" .json); [[ $anc == *" $pid "* ]] && continue; kill -0 "$pid" 2>/dev/null || continue
  t=$(find ~/.claude/projects -maxdepth 2 -name "$(jq -r .sessionId "$f").jsonl" -print -quit); [ -n "$t" ] && live+=("$t")
done
git worktree list --porcelain | awk '/^worktree /{w=$2} /^branch /{print w, $2} /^detached/{print w, "-"}' |
while read -r w ref; do
  [[ $w == "$main" || $w == *-slots/* || ! -d $w ]] && continue
  busy=; for p in /proc/[0-9]*/cwd; do [[ $(readlink "$p" 2>/dev/null)/ == "$w"/* ]] && { busy=1; break; }; done
  [ "$busy" ] && continue
  for t in "${live[@]}"; do grep -qF "${w##*/}" "$t" && { busy=1; break; }; done
  [ "$busy" ] && continue
  b=${ref#refs/heads/}
  [[ $b == concept/* && -z $dry ]] && git -C "$w" push -q origin "$b" 2>/dev/null
  head=$(git -C "$w" rev-parse HEAD)
  git merge-base --is-ancestor "$head" main || [ -n "$(git branch -r --contains "$head" 2>/dev/null)" ] || continue
  if [ "$dry" ]; then echo "would remove $w ($b)"; else git worktree remove "$w" 2>/dev/null && echo "removed $w ($b)"; fi
done
git worktree prune

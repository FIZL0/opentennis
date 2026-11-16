#!/usr/bin/env bash
# Add a plan/TODO.md task from a plain sentence, cheaply: one `claude -p` with ~2k tokens of context (code map, source
# file names, two example lines, open task subjects; no tools, no CLAUDE.md) turns it into a task line with its files,
# then `agent-runner plan add` puts it on top as the next free B id. If the note also asks to change existing tasks ("drop the
# cursor todo", "B319 should also…"), the model emits DROP/NOTE lines for those IDs: DROP ticks it into DONE.md marked
# dropped, NOTE appends to it. A note that only changes old tasks adds nothing new. Commits plan/TODO.md + DONE.md. Safe while a runner works: no merge in
# progress → its own commit; otherwise it stays uncommitted and the runner stashes it around its merges.
#
#   tools/todo.sh the umpire label should just say the name     (or no args: it asks, so quotes/apostrophes are fine)
#   HST_TODO_RAW=1 tools/todo.sh …                             skip the model, add the sentence as written
set -u
export AGENT_RUNNER_NOCOMMIT=1  # plan edits go into this script's own commit
root=$(cd "$(dirname "$(realpath "$0")")/.." && pwd)
cd "$root" || exit 1
text="$*"
[ -z "$text" ] && read -e -r -p 'todo: ' text
[ -z "$text" ] && exit 1
day=$(date +%F)
line=''
if [ "${HST_TODO_RAW:-0}" != 1 ]; then
  ctx="Code map:
$(awk '/^## Code map/{f=1} /^## Needs the human/{f=0} f' PLAN.md)

Source files:
$(git ls-files 'crates/*.rs' 'web/*' | grep -v /tests/)

Example task lines:
$(grep -m2 '^- \[ \]' plan/TODO.md | sed -E 's/^- \[ \] \*\*[^*]+\*\* //')

Open tasks (ID subject…):
$(grep -E '^\s*- \[[ ~]\] \*\*' plan/TODO.md | sed -E 's/^\s*- \[.\] \*\*([^*]+)\*\* /\1 /; s/^(.{0,90}).*/\1/')"
  # run outside the repo so no CLAUDE.md/memory loads; no tools, short system prompt
  reply=$(cd "$(mktemp -d)" && printf '%s' "$ctx" | timeout 120 claude -p --model sonnet --tools '' \
    --setting-sources '' --strict-mcp-config --no-session-persistence \
    --system-prompt "You turn the user's note into edits of a Rust/Bevy game's TODO list. Reply with lines only, one per edit:
ADD <task line> — for new work, in the examples' style: a short subject, a colon, what's wrong and what it should do (clear, specific, the user's meaning only, invent nothing), then ' (user $day)', then ' → ' and the 1–3 most likely files from the list, paths shortened as in the examples, joined by ' · '. Usually exactly one ADD; none if the note only changes existing tasks.
DROP <ID> <why, a few words> — only when the note asks to remove/cancel an existing open task and one clearly matches.
NOTE <ID> <text> — only when the note asks to change/extend an existing open task: the change, phrased as an addition.
Never put requests about existing tasks into the ADD line." \
    "Note: $text" 2>/dev/null)
  line=$(printf '%s\n' "$reply" | sed -nE 's/^[-*` ]*ADD:? +//p' | head -1 | sed -E 's/ +$//')
  printf '%s\n' "$reply" | sed -nE 's/^[-*` ]*(DROP|NOTE):? +([A-Z]+[0-9]+[a-z0-9]*):? +(.*)$/\1 \2 \3/p' > "${edits:=$(mktemp)}"
fi
while read -r op tid rest; do
  if [ "$op" = DROP ]; then
    agent-runner plan note "$tid" "dropped (user $day): $rest" >/dev/null && agent-runner plan tick "$tid" >/dev/null && echo "dropped $tid: $rest"
  else
    agent-runner plan note "$tid" "$rest (user $day)" >/dev/null && echo "noted $tid: $rest"
  fi
done < "${edits:-/dev/null}"
[ -n "${edits:-}" ] && [ -s "$edits" ] && [ -z "$line" ] && { [ -e .git/MERGE_HEAD ] || git commit -q -m "TODO: edit ${text:0:60}" -- plan/TODO.md plan/DONE.md 2>/dev/null || echo "(left uncommitted)"; exit 0; }
[ -z "$line" ] && line="$text (user $day)"
out=$(agent-runner plan add B "$line") || exit 1
id=${out#added }
[ -e .git/MERGE_HEAD ] || git commit -q -m "TODO: $id ${text:0:60}" -- plan/TODO.md plan/DONE.md 2>/dev/null || echo "(left uncommitted)"
echo "$id: $line"

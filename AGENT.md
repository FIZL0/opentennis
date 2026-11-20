# Agent notes

Rust/Bevy remaster of Hot Shots Tennis (SCUS-97610). Faithful: same AI, physics, timing; better rendering. PLAN.md
has the rules and where code lives; this is how to work. How-tos on demand: `agent-runner plan "<section>"` (from
`plan/GUIDE.md`): *PS2 libraries*, *Research setup*, *Function oracle*, *Tests in detail*, *Waiting on things*,
*Driving the game*, *Unattended runs*, *Mod versions*, *Concept branches*, *Main checkout in detail*.

## Never

- Commit game data, extracted assets or dumps: retail data is read from the user's ISO at runtime.
- Put retail addresses or Ghidra names (`FUN_…`, `DAT_…`) in shipped code; ports go spec first, then a second session
  ports from the spec with the decompile locked (PLAN.md *Clean-room ports*). Exceptions (user decisions):
  `hst-data::exe` reads tuning *data* from the user's own GAME.BIN at documented offsets after checking the disc is
  SCUS-97610 US 1.00, no code addresses (2026-10-06); a small curated set of promo screenshots of our own renderer in
  `docs/screenshots/`, not texture sheets, raw captures or bulk dumps (2026-10-15).
- Copy `../ps2-float` / `../ps2-oracle` back in: they're ours, fix them there (*PS2 libraries*).
- `rm` with a glob: the rm check prompts even in bypass mode and unattended runs sit on it; write each run to a fresh
  `context/<task>/runN/` or let the tool overwrite.
- `pkill pcsx2-qt`, or touch PCSX2 while a capture (`pgrep -f record_p2m2`) runs (*Driving the game*).
- `git add -A/./-u` or `git commit -a` in the main checkout (below).

## Ports

- Exactness: ported math uses `hst_sim::ps2` (PCSX2's EE FPU model: chop rounding, aligned adds) wherever the
  original's float results must match bit for bit.
- Gameplay ports are verified against the real game over PINE (`tools/pine.py`, launch with `tools/pcsx2-hst.sh`); a
  pure function of its arguments and RAM against the function oracle instead (*Function oracle*).
- Research: our own scripts and notes in `research/` (in git: no game data, no decompiled code, no captures or
  screenshots); anything from the disc or the running game in `context/` (scratch) or, if a test reads it,
  `fixtures/` (both git-ignored; *Research setup*).
- Journal: `research/journal/YYYY-MM-DD-<slug>/` (slug starts with the lowercase task ID), one dir per task: a
  `README-<STATE>.md` or numbered iteration files `N-<TOPIC>-<STATE>.md`; the newest one's state says whose move it
  is: `-FINAL` done, `-PART` agent continues, `-READY` needs the human. `tools/done.sh` renames a ticked task's newest
  `-READY`/`-PART` to `-FINAL`. Dirs with no state suffix count as `-FINAL`; the plan's checkbox wins if they
  disagree. Captures and images go in `context/`, not the journal.

## Plan

Open tasks in `plan/TODO.md` (line fields: `| files: a, b | after: X | test: t | journal: d`), finished in
`plan/DONE.md`, background (game control, done list, known gaps) in `plan/REFERENCE.md`. Never read or edit TODO.md
directly: `agent-runner plan` (index) · `next` · `<ID>` · `brief <ID>` · `"<section>"` · `-f FILE <name>` · `tick` ·
`block <ID> <why>` · `unblock <ID> [note]` · `move <ID> top|before ID|after ID|in SECTION` · `add <PREFIX> TEXT [@ WHERE]` ·
`note <ID> TEXT` · `newid <PREFIX>` · `drop <ID> WHY` (`--help`: the rest). "continue" means: `agent-runner plan next`,
read its brief, port + verify, `tools/done.sh <ID> "<msg>"` (tests, tick into DONE.md, commit).

New tasks name their files and waits as fields, never in prose (prose is never read for files or ids):
`agent-runner plan add B "text" --files crates/x/src/a.rs,crates/y/** --after B646 --test name`. A subtask (`add B646 …`
→ `B646a`) only when it can't start before its parent lands (needs its code, data or verdict): the runner runs one
family at a time. Anything else gets a fresh number, so it runs in parallel (user 2026-11-16); `after:` is the only way
to wait on a fresh-numbered task. Concept tasks: *Concept branches*.

## Release notes

Anything a player would notice (new feature, fix, change in how something plays or looks) gets one plain-English `- `
line in `RELEASE_NOTES.md`, committed with the change: what changed for the player, not how. No task IDs, crate names or
jargon. Internal work (tests, tooling, refactors, research) gets nothing. `tools/release.sh` publishes it. A published
mod's change bumps its `mod.json` `version` (*Mod versions*).

## Cheap context

- Every turn re-reads the whole conversation: pipe long output through `tail`/`grep`, read files by line range, never
  dump whole logs or big files.
- Tests: `tools/check.sh -p <crate> <filter>` for what you touched, not bare `cargo test`; the full suite runs once,
  in `tools/done.sh`. Tests read the disc only through `hst_data::fixture`, stay under ~10 s, and netplay has its own
  switch (*Tests in detail*).
- Background commands re-invoke you when they exit: never poll or `sleep; cat` (*Waiting on things*).
- Wrap-up is one call, no subagent: `tools/done.sh <ID> "<msg>"` or `… --blocked "<why>"`.

## Main checkout

Other sessions and the runners' merges share it. Commit only your own paths (`git commit -m "…" -- <paths>`; with
done.sh add `-- <paths>`), as soon as they're solid. `.git/MERGE_HEAD` exists → wait, don't commit or touch the index.
Bigger work while runners go: your own worktree (`git worktree add "$(agent-runner home)/wt/<slug>"`), merged when
done; never a runner slot (*Main checkout in detail*).

## Stuck

Never wait or loop on one obstacle: tool hangs, PCSX2 won't cooperate, the same fix fails twice, it needs the human, or
~20 minutes without progress → journal what happened and what you tried, commit what's solid,
`tools/done.sh <ID> "<msg>" --blocked "<why + what unblocks>"` (or `agent-runner plan block`), next task. PLAN.md
*When blocked* has the rest.

Tasks: this repo is worked by agent-runner. Use the agent-runner skill (or run `agent-runner guide`): queue work with `agent-runner plan add`, never edit the plan's TODO.md by hand, never merge task/* or wip/* branches. Asked to do something now? Use `agent-runner take "what" --kind K` (a worktree of your own; `--here` brings edits already made along); never edit the main checkout.

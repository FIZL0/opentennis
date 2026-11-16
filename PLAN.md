# PLAN — Hot Shots Tennis remaster

The rules. Tasks are in `plan/TODO.md` (open) and `plan/DONE.md` (archive): read this, then **only** the task's own
file (`agent-runner plan <ID>`) and the files listed on its line.
Don't grep the repo to find where things live — the code map says (`agent-runner plan <crate>`). Re-read files rather than remember;
other sessions change them.

## Continue

"continue" = take the first open task (`agent-runner plan next`), read `agent-runner plan brief <ID>` (its line, plan file,
journal and files in one go; the runners paste it into your prompt already), carry it out completely, write the journal
entry, then `tools/done.sh <ID> "<commit message>"` (tests → tick → commit in one call), stop and report. One task per
iteration; don't merge or skip ahead. Too big → split it into sub-tasks (new `plan/tasks/<ID>.md` files + lines in `plan/TODO.md`) first.

- **Match the original exactly** — no approximations or placeholders. Find it in the decompile (`context/decomp/`,
  `research/fn.sh <addr>`), port it, prove it against the real game: a test compares the port to a PINE recording
  or save-state capture and passes bit-exact (f32 via `hst_sim::ps2`) or frame-exact.
  This covers gameplay only: physics, AI, shots, scoring, rules and match timing.
- **Clean-room ports: spec, then port, in two sessions** (user 2026-10-19). A session that reads the decompile to
  port behaviour writes no Rust: it writes `research/journal/<dir>/SPEC.md` (in the task's own journal) and ends with
  `tools/done.sh <ID> "<msg>" --spec`. The spec is behaviour in your own words: inputs, outputs, rules, constants by
  meaning and value, the order things happen in, the quirks the game shows (those are behaviour, match them). It
  has no code, no addresses, no Ghidra names, no struct offsets or field order (`tools/audit.sh` checks), and ends with
  `## Open questions` (`none`). The next session gets a *Port phase* brief: it implements from the spec and the
  recordings/oracle, with the decompile locked (`.cleanroom`, `tools/guard_decomp.py`); a gap goes under *Open
  questions* and back through `--spec`. Write the port as you'd write it fresh: your own structure and names, no dead
  branches, unused fields or instruction order copied over; the tests prove it's exact. Why: a port transcribed from
  decompiled code is a copy of it (CREDITS.md, *How it's made*).
- **Rendering only has to look right** (user 2026-10-12): shadows, shade maps, lighting, effects and GS output
  match the original by eye or within a small tolerance against a capture, not bit for bit. Don't replay GS/VU1
  rounding to close the last few pixels, and don't open a follow-up "(X gap)" task just to chase remaining
  bit-level render differences: note them in the journal and move on.
- **The main menu and front-end don't have to match the original** (user 2026-10-12): title, mode menus, character/
  court/umpire select, settings, loading and result screens are ours to design. The original is a reference for
  them, not a spec; no bit-/frame-exact proof needed. Where a task in `plan/TODO.md` says a menu must look or behave
  "like the original" / "1:1", treat that as a starting point, not a requirement.
- **In-match presentation only has to feel right** (user 2026-10-12): instant replays (when they trigger and how
  they play), cameras, cutaways, crowd/umpire animation, commentary and SFX cues match the original by eye. No oracle
  or PINE proof; just check it doesn't touch the sim RNG or match timing. Oracle runs stay at ≥10k inputs; no
  mutation testing unless a test looks too weak.
- **Additions are layered on top** (character/texture mods, other games'
  characters, widescreen, upscaled textures): with them off or unused, a match must still play
  exactly like the original (the replay tests stay bit-exact). A mod character uses the same physics and AI code
  paths as a stock one, fed its own data.
- **PS2 arithmetic**: EE FPU chops, masks the smaller addend to exponent diff − 1 guard bit; `madd.s` = that add of
  acc + truncated product. Use `hst_sim::ps2` / `hst_sim::vu0`.
- **Game data from the user's disc at runtime**, never committed (`hst_data::iso`, `xb`, `exe` — only `exe` knows
  retail data offsets, disc-ID checked). Characters too (`crates/hst-base/src/character.rs`).
- **Same input, same result**: P0's replay harness (`hst_sim::replay`) is the acceptance test for gameplay.
- **Handedness**: read it from character data (Carol, Will lefty); mirror forehand/backhand, reach, swing/serve,
  toss hand, racket.
- **All UI at any aspect ratio** (user 2026-10-08): every HUD element, pop-up, menu, result/loading screen and
  new screen must lay out for any aspect ratio and resolution, pinned/scaled like B17's panels
  (`hst-base/src/widescreen.rs`), never locked to 4:3 or one resolution. Scale uniformly, never stretch (user 2026-10-13): an element keeps
  its own aspect at any window size; extra space goes around it. Any UI task isn't done until it is.
- **Look at the original too**: `tools/screenshot.sh` (PCSX2) vs `--shot out.png` at the same moment for anything
  visual; screenshots go in `context/`, not the journal.
- Findings → `research/journal/<date>-<slug>/`. When a task lands: tick it, add to `plan/REFERENCE.md` *Done* /
  *Play it now* if it changed.
- Game control (PINE, vpad, save-state slots 3/4/5 = user's, load only; scratch 8/9): `plan/REFERENCE.md`.

## When blocked

Stalled (tool hangs, PCSX2 won't cooperate, same fix fails twice, ~20 min on one obstacle, needs the human):
stash experiments, write details in the journal and `tools/done.sh <ID> "<msg>" --blocked "<why + what unblocks>"`.
PCSX2 runs unattended (`tools/pcsx2-hst.sh`, `tools/screenshot.sh`, PINE): "couldn't capture / not launched" is not a
blocker, launch it; move to the next `- [ ]` — preferably another section. Never end a
run while `- [ ]` lines remain; at the end list blockers. Human-only items go in `plan/HUMAN.md`, never into
code as guesses. Don't touch PCSX2 while `pgrep -f record_p2m2` runs.

## Code map

`agent-runner plan <crate>` (hst-data, hst-sim, hst-oracle, hst-base, hst-play, hst, hst-gltf, tools, docs), from
`plan/CODEMAP.md`; add or fix its line when a module lands or moves.

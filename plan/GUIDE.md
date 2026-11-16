# Guide

How-tos AGENT.md points to, read when the work needs them: `agent-runner plan "<section>"` prints one.

## PS2 libraries

FIZL0/ps2-float (`../ps2-float`: EE FPU, VU0, the SDK's fdlibm, newlib rand/MT; `hst_sim::ps2`/`vu0`/`libm`/`rng`
re-export it) and FIZL0/ps2-oracle (`../ps2-oracle`: the R5900 interpreter under `hst-oracle`) are ours: tag-pinned
git dependencies shared with the decomp kits. Agents may fix and extend them when a port needs it (new op, missing
function, wrong rounding): change it there with a test, tag a release, bump the tag here and in the decomp kits (their
own repos' `crates/game-sim`, `crates/kit-oracle`, not in this one), run their tests. Never copy them back in.

## Research setup

`research/README.md` rebuilds `context/` and `fixtures/` from the disc (extract, overlay ELFs, Ghidra project,
decompilation dumps, save states, recordings) and lists the tool pitfalls. `fixtures/` (git-ignored, game data) holds
everything the tests read: disc extracts (`fixtures/iso`, `xb`), RAM images, recordings. A recording a test needs is
copied into `fixtures/<task>/` (`cp --reflink`) when the test lands; tests never read `context/`.
`tools/context_prune.py` reports what `context/` holds that nothing refers to any more (`--delete` moves it to
`context/.trash/`, `--empty` clears that); a recording a research script needs must be named by its path in it.

Decompile: `research/fn.sh <addr>` (one function), `research/xref.py <addr>` (callers/callees),
`research/xref.py <addr> --similar` (look-alike functions; `ported` ones have Rust to copy the shape of). A hook
blocks reading the 16 MB dumps whole.

## Function oracle

`crates/hst-oracle`: for a pure function of its arguments and RAM (math, AI decisions, table lookups) check the port
with `Ee::game()`/`Ee::from_ram(dump)` + `call(addr(name), ints, floats, budget)` over ≥10k generated inputs instead of
recording it (names → addresses in `research/oracle.txt`; examples in its `tests/`; `use hst_oracle::HstEe` for those
two). An unknown op panics with its address: add it to the interpreter (ps2-oracle, *PS2 libraries*). Timing, frame
flow, VU0 and rendering still need PCSX2 recordings.

## Tests in detail

`tools/check.sh [-p crate --test name]`, not bare `cargo test`; full log in `context/notes/check.log`. Three check
slots serve every agent, and a full-suite run holds one for many minutes: while working run only the tests you
touched (`tools/check.sh -p hst <filter>`); the full suite runs once, in `tools/done.sh`.

A test reads the disc and `fixtures/` recordings through `hst_data::fixture` (`iso()`, `read`, `skip`), never by its
own path: it skips without them, and fails instead under check.sh's `HST_REQUIRE_DISC=1`.

Keep each test under ~10 s: share one match between checks, force the rare case (e.g. set the replay camera's
closing-shot list) instead of playing on until it happens. Slower ones are `#[ignore]` with a doc line saying how and
when to run them (`tools/check.sh -p hst <name> -- --ignored`).

Netplay: `check.sh` skips the netplay match tests (`play::netplay::tests::matches`) unless `HST_NETPLAY=1` or the
filter names netplay/rollback/synctest. Touch netplay, net or the snapshot → `HST_NETPLAY=1 tools/check.sh -p hst-play
netplay` (the match, netplay included, is in `hst-play`; the menus in `hst`). Ignored: the full-match netplay/rollback
runs, `synctest_through_a_change_of_ends` (snapshot/change of ends), `a_silent_peer_rejoins_and_a_silent_host_ends_it`
(`recover.rs`), `short_code_on_a_testnet` (DHT/short codes). Touch those areas → run the matching one.

## Waiting on things

A `run_in_background` command re-invokes you when it exits, so end the turn or do other work. Don't
`sleep N; cat …output` (a hook blocks it) and don't poll its output file. If you really must wait on a condition, use
a bounded loop (`timeout 600 bash -c 'until …; do sleep 3; done'`) that also stops when the process exits.

## Driving the game

`plan/REFERENCE.md` → *Controlling the real game* (PINE, virtual pad `tools/vpad.py`, `tools/screenshot.sh`; remaster
`--shot`s in a fixed-size floating window: `tools/shot.sh`) and `PLAN.md` → *When blocked*. Multi-step game drives go
under `tools/pcsx2.sh <cmd>`; pine.py tools take the same lock by themselves. In a parallel run `HST_PCSX2=N` gives
each agent its own PCSX2 copy (tools, pad, PINE and lock all follow it). When you're done with the game for your task,
close yours: `tools/pcsx2.sh tools/pcsx2-hst.sh stop` (waits for anything mid-use). Never `pkill pcsx2-qt`.
`CAPTURE FAILED` from any PINE tool means the output file is incomplete: fix PCSX2, re-record. Don't touch PCSX2 while
a capture (`pgrep -f record_p2m2`) is running.

Wine (Windows build checks): always `WINEDLLOVERRIDES="mscoree,mshtml=" WINEDEBUG=-all wine …` so a fresh prefix
doesn't pop the mono/gecko install dialogs on the user's desktop.

## Unattended runs

`tmux new -s hst agent-runner run [N|once]` ([agent-runner](https://github.com/FIZL0/agent-runner), settings in
`runner.toml`; N slots at once in worktrees, default 5, `once` = one task then stop (never two tasks on one file);
`--force` = past the 94% weekly cap; `--shutdown` = power the computer off once nothing is
running, merging or startable; stop gently with s + Enter in its pane or `agent-runner stop`; each slot builds in its
own worktree `target/`, dependencies shared through sccache; don't set `CARGO_TARGET_DIR`).

## Mod versions

Any change to a published mod (rebuilt model, motions, faces, textures, manifest fix) bumps its `mod.json` `version`
before it is re-uploaded, so the mod store sees an update and installed copies refresh.
`../opentennis-mods/out/mods` is the user's live mods folder, and a mods worktree that links `out/` to main's writes
straight into it: merge main into your branch before running a packager there (`package.py`, `texpack.py`, court
exports refuse otherwise, `tools/fresh.py`), or a stale branch rebuilds the mods without main's newer work.

## Concept branches

A task marked **concept** (user 2026-10-21) is an experiment the user wants to try before it reaches main: its code
lives on `concept/<slug>` (the slug is on the task line), never on main. Pick up where the branch left off
(`git fetch origin concept/<slug>` → `git switch concept/<slug>`, or `git switch -c concept/<slug>` the first time;
merge main into it) in your own slot, not a new `git worktree add` folder (`tools/worktree-gc.sh` sweeps any left over
after each runner session), build and commit there, `git push -u origin concept/<slug>`, then `git switch` back to
your task branch so it carries only the journal and plan. Write in the task's journal what's on the branch, how to try
it (`git switch concept/<slug>`, the flags) and what's next, then end with `tools/done.sh <ID> "<msg>" --blocked
"concept round N on concept/<slug>: waiting on the user"`. The user unblocks it for another round, or asks for a merge
into main as a normal task. Mode-off and the replay tests stay exact on the branch too.

## Main checkout in detail

Other sessions and the runners' merges work in the main checkout at the same time. Commit as soon as a change is
solid; uncommitted work can end up stashed or in someone else's commit. `.git/MERGE_HEAD` exists → a merge is under
way: wait for it, don't commit or touch the index. A runner merge waits while uncommitted edits in main touch the files
it changes. Anything bigger than a quick edit while runners are going: your own worktree
(`git worktree add "$(agent-runner home)/wt/<slug>"`, or EnterWorktree / `claude -w <slug>` for the user's interactive
session once it moves from adding TODOs to code), merged into main when done. Never use a runner slot
(`$(agent-runner home slots)/sN`): the runner resets those. `tools/done.sh` (`git add -A`) is for the runners' task
worktrees; in main add `-- <paths you changed>` and it commits only those + plan/. A hook blocks `git add -A/./-u` and
`git commit -a` there. The `chore` subagent is only for bigger mechanical jobs (checking a capture, reformatting).

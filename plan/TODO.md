# TODO — open tasks, in order

Rules: `PLAN.md`. Finished tasks move to `plan/DONE.md` (`agent-runner plan tick <ID>`).

Each line ends in fields: `| files:` the files it touches (mode d keeps running tasks apart by them) · `| after:` ids done first · `| test:` tests · `| journal:` dir under `research/journal/`. Add with `agent-runner plan add P "text" --files a,b --after X`. Tests: hst-sim,
hst-data and hst-oracle in the crate's `tests/`; hst-base, hst-play and hst in-module (`#[cfg(test)]`), no `tests/` dir. Fixtures in `fixtures/` (`match_s05.bin` slot-5 bot match, `round1.bin` doubles,
`new_recording.bin` dives/smashes/collisions).

### Next (user priorities, in order)

1949cf8c6b5b557f/bevy_render-0.19.1/src/render_phase/mod.rs:1241:40:
index out of bounds: the len is 0 but the index is 0"

- [ ] **T31** Slots test only what they touched; the combined merge runs everything (the person agreed, 2026-11-24). Why: tools/done.sh runs the whole suite, clippy on all targets and the browser check in every slot, and the runner's merge gate then runs the whole suite again on the combined tree, so every worktree fills with builds. 1. Move clippy (--workspace --all-targets -D warnings) and tools/webcheck.sh, with their messages, from done.sh into a new tools/gate.sh that runs tools/check.sh, then those two; runner.toml [run] test = tools/gate.sh; keep retest on tools/check.sh, and keep tools/gate.sh, check.sh and webcheck.sh out of [run] untested, so a change to one is gated. 2. done.sh in a worktree keeps audit.sh, patches.py and tests only the crates its diff against main touches (check.sh -p <crate>; none touched: no tests), in --blocked too; in the main checkout, where no merge gate follows, it runs tools/gate.sh as now. 3. AGENT.md, PLAN.md, tools/runner-prompt.md and done.sh's header say so. Check: done.sh on a one-crate change builds no other crate's test binaries; a clippy warning or web-only break fails tools/gate.sh; a commit that fails another crate's test is caught by the merge gate. | files: tools/done.sh, tools/gate.sh, runner.toml, AGENT.md, PLAN.md, tools/runner-prompt.md
### Netplay and mobile (user 2026-10-09: right after the bugs/priority list, in this order)

### Match basics

### Shots

### Players


### AI

### Match

### Presentation


### Confirmations

### Follow-ups

- [ ] **T31a** (follow-up, normal) The merge gate runs everything: new tools/gate.sh runs tools/check.sh, then clippy as tools/done.sh does (--workspace --all-targets -D warnings, same messages), then tools/webcheck.sh; done.sh calls gate.sh in place of those three for now, so no check is lost. runner.toml [run] test = tools/gate.sh; retest stays on tools/check.sh; gate.sh, check.sh and webcheck.sh are kept out of [run] untested. Check: a clippy warning or a web-only break fails tools/gate.sh; a failing test still prints the lines [run] failures matches. | files: tools/gate.sh, tools/done.sh, runner.toml
- [ ] **T31b** (follow-up, normal) Slots test only what they touched: in a worktree done.sh keeps audit.sh, patches.py and tests only the crates its diff against main touches (check.sh -p <crate>; none: no tests), in --blocked too; in the main checkout, where no merge gate follows, it still runs tools/gate.sh. AGENT.md, PLAN.md, tools/runner-prompt.md and done.sh's header say so. Check: done.sh on a one-crate change builds no other crate's test binaries; another crate's broken test is caught by the merge gate. | files: tools/done.sh, AGENT.md, PLAN.md, tools/runner-prompt.md | after: T31a


### Stretch

- [~] **P6** Serve details, lets — user 2026-10-16: get a let serve by driving the real game with the virtual pad (tools/vpad.py, as P11k4/k5 do), recording with PINE; vary the server's character, serve power/aim and position until a serve clips the net cord and lands in the box. Then port and verify the let path against that recording. Time limit: stop after 2 hours of trying for a let; if none, write what was tried in the journal and re-block with that — user 2026-10-16: don't play rallies out: save a state just before the serve (scratch slot 8/9) and reload it after each serve that isn't a let, then serve again with a new character/power/aim/position — user 2026-10-16: Carol probably can't hit a let; serve with other characters — user 2026-10-16: try the expert characters for the let | files: crates/hst-sim/src/serve.rs, crates/hst-play/src/play.rs, crates/hst-sim/src/ball.rs | test: serve.rs BLOCKED: user 2026-11-16: keep blocked until the user unblocks it
- [~] **M133** **Debug clip bind** (user 2026-10-19, dev builds only): while a match plays keep a ring of snapshots (one a second, last 60, `play/snapshot.rs`) plus every tick's pads; a bind (F9) writes the oldest snapshot + the inputs since to `context/clips/<time>.clip`. A test helper loads a clip and replays it headless frame by frame (per-tick `state_hash` checks it replays the same), so a bug report like the CPU partner not hitting an easy ball can be replayed with probes attached. Not in release builds. | files: crates/hst-play/src/play/snapshot.rs, crates/hst-play/src/play/tick_input.rs, crates/hst-play/src/play.rs BLOCKED: user 2026-11-16: keep blocked until the user unblocks it
- [~] **P126** Practice-mode exceptions (T8; only the one-player ball machine of M2l reaches them): practice's side pick between aim candidates and its red-marker timeout, the shot effects' practice-only exception, the swing's practice exception (solo with player 1 hitting). Port them for the lobby ball machine | files: crates/hst-play/src/play.rs, crates/hst-sim/src/shot.rs, crates/hst-sim/src/swing.rs | journal: 2026-10-21-repo-audit BLOCKED: user 2026-11-16: keep blocked until the user unblocks it

### Lowest priority (user 2026-10-07: creatures, NPCs, ball-hit obstacles after everything else)

Done tasks: one line each in `plan/REFERENCE.md` *Done*, full text in `plan/DONE.md`.

### Polish (at the end, user 2026-10-08)

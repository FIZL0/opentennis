# TODO — open tasks, in order

Rules: `PLAN.md`. Finished tasks move to `plan/DONE.md` (`agent-runner plan tick <ID>`).

Each line ends in fields: `| files:` the files it touches (mode d keeps running tasks apart by them) · `| after:` ids done first · `| test:` tests · `| journal:` dir under `research/journal/`. Add with `agent-runner plan add P "text" --files a,b --after X`. Tests: hst-sim,
hst-data and hst-oracle in the crate's `tests/`; hst-base, hst-play and hst in-module (`#[cfg(test)]`), no `tests/` dir. Fixtures in `fixtures/` (`match_s05.bin` slot-5 bot match, `round1.bin` doubles,
`new_recording.bin` dives/smashes/collisions).

### Next (user priorities, in order)

1949cf8c6b5b557f/bevy_render-0.19.1/src/render_phase/mod.rs:1241:40:
index out of bounds: the len is 0 but the index is 0"

### Netplay and mobile (user 2026-10-09: right after the bugs/priority list, in this order)

### Match basics

### Shots

### Players


### AI

### Match

### Presentation


### Confirmations

### Follow-ups

- [~] **M183a1** (follow-up, normal) EU 50 Hz: state-for-state bot match in play, then the flip. Play the European bot match in play from the original's seed (Seed::after(875476), doubles chars 0/1/1/0, court 6, 1 set of 4 games: the ignored probe play::fifty_hz::tests::the_european_bot_match_plays_as_recorded) and make it match fixtures/m169/eu-bot-match.bin phase for phase and then tick for tick (players, ball, score). As of M183a the first serve set-up already differs (135 recorded vsyncs vs 215 in play) and no US match was ever compared this way in play: first get a US baseline the same way (slot 5's match_s05 recording, its seed by walking rand() back as P3d3 did), so 50 Hz gaps can be told from general ones; gaps that need the decompile go back through a spec. Then flip match_speed::FIFTY_HZ_SIM and add the release note (journal 2026-11-16-m183a-eu-50hz-rest) | files: crates/hst-play/src/play/fifty_hz.rs, crates/hst-play/src/play.rs, crates/hst-base/src/match_speed.rs, RELEASE_NOTES.md | test: fifty_hz — reopened: M184 landed — reopened: M186 landed — reopened: M190, M185 landed — reopened: B719 landed — reopened: B723 landed — reopened: B727 landed — user 2026-11-22: install PCSX2 v2.9.52 beside the current one for the slot copies so states 3/4/5 load (see plan/HUMAN.md) — reopened: B766, R14 landed — reopened: B768, B769 landed — B772 answered: the per-swing EU racket lead is B773, the 1588 dive B774 — reopened: B771, B773, B774 landed — part 11: with B782's stand-in the EU probe matches 20 phases; still undiagnosed: point 1's players part at tick 571, and point 4's rally (phase 20) runs 445 ticks in play against 398 recorded (journal part 11) BLOCKED: (after B782) needs B782 (spec: the 50 Hz intro tick count and when the match setup spawns clouds); then the parts at tick 571 and point 4's rally, tick for tick, the flip and the release note
- [ ] **B781** (follow-up, normal) EU racket-z fit ticks 179 and 538 (B773's numbers, rally window 38056–39692): on the tick player 1 switches from turn_fore to a stroke motion (0x12, 0x10), the doubles stand test's same-tick read already plays the new motion (179: play −0.8610) while the game's read still sees turn_fore one frame on (−0.7908). New since B777; B774 (first step on the press tick) landed in between. Find which tick the game's stroke start takes effect in player 1's update relative to player 2's read and match it. Journal: research/journal/2026-11-23-b779-in-fade-residual/. | files: crates/hst-play/src/play/doubles_ai/scratch.rs, crates/hst-play/src/play.rs | after: B779
- [ ] **B783** (follow-up, normal) US bot-match probe's held ball on recorded tick 0 (the first set-up tick, server still in the 0x20 serve stance, before the 0x20 to 0x21 crossfade) is one stance frame ahead of the recording (game [2.9440882, -0.5307655, -12.321224], play [2.9468277, -0.5111898, -12.317473]). Seeding the server's stance at clock 1/sampled 0 instead of B775's captured 2/1 makes tick 0 match but tick 1 then lags a frame, so the game's stance seems to advance two frames between ticks 0 and 1, or its tick-0 ball is placed a frame stale. The EU probe matches at tick 0. Find why and start the_us_first_serve_set_up_plays_as_recorded's held check (serve_to_contact) at 0. Journal: research/journal/2026-11-23-b780-serve-crossfade-held-ball/ | files: crates/hst-play/src/play/fifty_hz.rs | after: B780



### Stretch

- [~] **B72** Post-point replays still bad and blue screens still happen (bug, user 2026-10-08; moved to stretch 2026-10-12: replays later; B22/B25 didn't cover it all): after some points the replay/cut-away is wrong and the screen goes plain blue (nothing drawn). Reproduce (bot matches over many points, all courts, log the cut-away chosen and when the frame is empty), find each cause and match the original's replay → `play/cutaway.rs` · `play.rs` — From dropped B127 (user 2026-10-12, dropped 2026-10-16, may already be fixed): after a game-winning point, partway through the replay the score went back to deuce and the match carried on as if the game was never won (the replay counted as a point?). Replays are off in the app until replays are fixed (instant_replay.rs REPLAYS, on in tests); when turning them back on, check the replay never touches the score (re-run scoring/point-end hook, stale snapshot restore, or replay sim state leaking into the live match) and add a test: a game-winning point + its replay keeps the game won — also show the original's replay marker during post-point replays (from B107, dropped 2026-10-21) | files: crates/hst-play/src/play.rs, crates/hst-sim/src/score.rs, crates/hst-sim/src/flow.rs | test: cutaway.rs BLOCKED: shelved by the user 2026-10-21: replays later — user 2026-10-22: when replays land, □ (pad West / Xbox X / Space) asks for the post-point replay again, the controls page names it so, and the on-screen pad shows □ after the point (hidden during serve and rally); B655 took replay off □ for now — bounce debris (from B680, user 2026-10-26): when replays land, port the replay-only bounce effects in research/journal/2026-10-26-b680-bounce-court-fx/SPEC.md marked 'deferred to B72' (grass blades par/siba + clay clods par/tuti on a replay's first bounce, marks restored at replay start, smash-burst size cut in replay close-up cams, 90 rand() calls per grass replay blade burst); still open: does a replay restore rand()'s state afterwards (check rand() before/after a forced grass-court replay)
- [~] **P6** Serve details, lets — user 2026-10-16: get a let serve by driving the real game with the virtual pad (tools/vpad.py, as P11k4/k5 do), recording with PINE; vary the server's character, serve power/aim and position until a serve clips the net cord and lands in the box. Then port and verify the let path against that recording. Time limit: stop after 2 hours of trying for a let; if none, write what was tried in the journal and re-block with that — user 2026-10-16: don't play rallies out: save a state just before the serve (scratch slot 8/9) and reload it after each serve that isn't a let, then serve again with a new character/power/aim/position — user 2026-10-16: Carol probably can't hit a let; serve with other characters — user 2026-10-16: try the expert characters for the let | files: crates/hst-sim/src/serve.rs, crates/hst-play/src/play.rs, crates/hst-sim/src/ball.rs | test: serve.rs BLOCKED: user 2026-11-16: keep blocked until the user unblocks it
- [~] **M133** **Debug clip bind** (user 2026-10-19, dev builds only): while a match plays keep a ring of snapshots (one a second, last 60, `play/snapshot.rs`) plus every tick's pads; a bind (F9) writes the oldest snapshot + the inputs since to `context/clips/<time>.clip`. A test helper loads a clip and replays it headless frame by frame (per-tick `state_hash` checks it replays the same), so a bug report like the CPU partner not hitting an easy ball can be replayed with probes attached. Not in release builds. | files: crates/hst-play/src/play/snapshot.rs, crates/hst-play/src/play/tick_input.rs, crates/hst-play/src/play.rs BLOCKED: user 2026-11-16: keep blocked until the user unblocks it
- [~] **P126** Practice-mode exceptions (T8; only the one-player ball machine of M2l reaches them): practice's side pick between aim candidates and its red-marker timeout, the shot effects' practice-only exception, the swing's practice exception (solo with player 1 hitting). Port them for the lobby ball machine | files: crates/hst-play/src/play.rs, crates/hst-sim/src/shot.rs, crates/hst-sim/src/swing.rs | journal: 2026-10-21-repo-audit BLOCKED: user 2026-11-16: keep blocked until the user unblocks it

### Lowest priority (user 2026-10-07: creatures, NPCs, ball-hit obstacles after everything else)

Done tasks: one line each in `plan/REFERENCE.md` *Done*, full text in `plan/DONE.md`.

### Polish (at the end, user 2026-10-08)

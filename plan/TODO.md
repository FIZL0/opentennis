# TODO — open tasks, in order

Rules: `PLAN.md`. Finished tasks move to `plan/DONE.md` (`tools/ctx.py tick <ID>`).

`→` files to open (play.rs fns by name) · `t:` tests · `j:` journal under `research/journal/`. Tests: hst-sim,
hst-data and hst-oracle in the crate's `tests/`; hst-base, hst-play and hst in-module (`#[cfg(test)]`), no `tests/` dir. Fixtures in `context/fixtures/` (`match_s05.bin` slot-5 bot match, `round1.bin` doubles,
`new_recording.bin` dives/smashes/collisions).

### Next (user priorities, in order)

- [ ] **P131** **Serve state's end** (P130): the original's CPU server leaves its serve state for the rally 31 frames after its serve's hit message (ai_glue_s05, both points: v7784→7815, v8841→8872); the app's server first steps its rally 55–56 ticks after the hit, once its serve follow-through lets go (`bot`'s serve_follow / ai_serve_stick). Spec the serve routine's end condition (clean-room: spec session first), port, check the 1 → 3 frame in glue_matches_the_game and an app test → crates/hst-play/src/play.rs bot/ai_serve_stick · play/doubles_ai.rs step | t: hst-sim ai_rally.rs, play/doubles_ai/serve_over.rs | j: 2026-10-07-p11-ai (27-P130)
- [~] **B559** Aloha Beach foliage explodes in the browser (user report, Brave, 2026-10-17): after a few rounds the plant/tree meshes' triangles stretch across the screen (static meshes, so their GPU vertex data got stomped; screenshot context/bugs/aloha-foliage-brave.png). Suspects: vertex_patch.rs writes (costume sway) landing in a stale/reused slab range after rig despawn or slab growth (third_party/bevy_render slab_allocator.rs reallocate_slab), or WebGL2's large-slab path. Repro: web build (and HST_WEBGL2=1 desktop) bot matches on Aloha over several games/rematches; skip the sway's vertex_patch::send to bisect; check WebGPU vs WebGL2 → crates/hst-base/src/vertex_patch.rs · noise.rs · third_party/bevy_render/src/slab_allocator.rs · crates/hst-play/src/stage.rs BLOCKED: no repro on Linux/ANGLE-Vulkan (WebGL2 + WebGPU full matches clean); needs the reporter's OS/GPU/Brave version, WebGPU vs WebGL2 and a console log, or a Windows Brave (ANGLE D3D11) run with a human in doubles
- [~] **B555** Player drawn wrong on Zen browser (user 2026-10-17, web build, Firefox-based): 3P (the cowboy) shows a second racket lying loose on the court beside him and a faint ghost racket at his hand (context/reports/2026-10-17/2.png). Looks like a racket/attachment bone matrix or skinning going wrong on Firefox's WebGPU/WebGL; check with Firefox, compare to Chrome on the same frame (crates/hst-base/src/character.rs, crates/hst-base/src/gs.rs) BLOCKED: not reproduced natively or in headless Zen with the report's court and lineup; needs the user's windowed Zen in an online 4-player match (does it persist, does a local match show it) — see journal 2026-10-17-b555-zen-racket
- [~] **B554** Black box on the court on Microsoft Edge (user 2026-10-17, web build, online doubles on court 4 — room court index 3): a solid black rectangle stands behind the net, centre court, with the net's glass-like panel beside it (context/reports/2026-10-17/1.png). Probably a prop or a transparent/shadow material whose texture or blend failed on Edge's WebGPU/WebGL backend. Reproduce (Edge or Chromium with the same backend, that court), find the mesh (cull/lod/gs/shade), fix it (crates/hst-base/src/gs.rs, crates/hst-base/src/textures.rs, crates/hst-play/src/stage.rs) BLOCKED: no repro: native court 4 has no such object and the web harness can't import the ISO on the new landing page; needs a working Chromium/Edge run or more detail from the user (offline? ?webgl2? from the first point or after the change of ends?)
- [~] **B510** Post-point camera inside an object (user 2026-10-16): a post-point camera sat inside something (court/view not yet given; ask the user for court + screenshot). B240's eye-buried test and B303's in_front fallback miss it. Reproduce with bot matches logging the drawn camera vs nearby props/umpire chair/court walls, compare with the original at the same moment, hide/move as the original does (else B303's fallback) → play/net_hide.rs · hst-sim/src/cutaway.rs · play/cutaway.rs | j: 2026-10-13-b240-near-camera-props BLOCKED: user 2026-10-16: it happened on a modded court (a mod's props/geometry, not stock); needs the mod court + screenshot from the user to reproduce — skipped by user 2026-10-16; agent stopped mid-work, WIP parked on wip/B510 (stash commit: net_hide.rs, stage.rs, main.rs; untracked files in its 3rd parent) — wip/B510 (TEMP HST_CAMLOG probe in net_hide.rs) deleted 2026-10-16; rewrite the probe if the mod court turns up — untracked B510 leftovers in the main checkout (crates/hst-data/tests/tmp_b510.rs, builds as a test; research/b510_chair_watch.py): park them on wip/B510 or delete (repo audit 2026-10-21)
1949cf8c6b5b557f/bevy_render-0.19.1/src/render_phase/mod.rs:1241:40:
index out of bounds: the len is 0 but the index is 0"
- [~] **B40g2** Offbeat Rules — **concept** `concept/offbeat-rules` (user 2026-10-21; AGENT.md *Concept branches*, never merge to main): the original's confirm-screen rules (messages 331, 343–346): rule 1 irregular bounce (`ball_irregular` model in CMN GAME.XB `hatsuyama/ball`, changed bounces), rule 2 slow motion (the global slow-mo; P0b4d is done); rules 3/4 are empty placeholders in the original. Round 1: port rules 1–2 on the branch, and write a proposal of new offbeat rules that would make matches fun (several ideas, each with how it plays, what it changes in the sim, cost) in the journal; prototype the best one or two on the branch behind the rules screen. With every rule off a match stays bit-exact → `play/main_menu.rs` · `hst-sim/src/ball.rs` · `play.rs` | j: 2026-10-08-b40-menus/2-B40G-DEFERRED.md BLOCKED: concept round 1 on concept/offbeat-rules: waiting on the user

### Netplay and mobile (user 2026-10-09: right after the bugs/priority list, in this order)

### Match basics
- [ ] **P125** Prop collision node walk off court 10 (T8): child nodes of a court prop are placed by local · parent, checked only on court 10 where every prop is one node. Check the collision world of a court with multi-node props against the game (RAM dump of the placed collision, or a recorded ball hitting such a prop), fix if it differs → crates/hst-sim/src/court.rs nodes · world.rs | t: hst-sim world.rs, collision.rs | j: 2026-10-21-repo-audit (T8 ledger)

### Shots

### Players


### AI

- [~] **P11** Opponent AI → `play.rs` bot/bot_serve/intercept/draw_due BLOCKED: umbrella (user 2026-10-21): the open AI work is P11k6, P11l6 and the review P116; everything the bot-only recordings reach is ported and bit-exact — P116 review (2026-10-21): table in journal 26-P116-REVIEW; gaps P127 (change of ends draws), P128 (app step replay), P129 (strike/new-path), P130 (serve hand-over order/aim frame)

### Match

### Presentation


### Confirmations


### Stretch

- [~] **B72** Post-point replays still bad and blue screens still happen (bug, user 2026-10-08; moved to stretch 2026-10-12: replays later; B22/B25 didn't cover it all): after some points the replay/cut-away is wrong and the screen goes plain blue (nothing drawn). Reproduce (bot matches over many points, all courts, log the cut-away chosen and when the frame is empty), find each cause and match the original's replay → `play/cutaway.rs` · `play.rs` | t: cutaway.rs — From dropped B127 (user 2026-10-12, dropped 2026-10-16, may already be fixed): after a game-winning point, partway through the replay the score went back to deuce and the match carried on as if the game was never won (the replay counted as a point?). Replays are off in the app until replays are fixed (instant_replay.rs REPLAYS, on in tests); when turning them back on, check the replay never touches the score (re-run scoring/point-end hook, stale snapshot restore, or replay sim state leaking into the live match) and add a test: a game-winning point + its replay keeps the game won → play.rs react/next_point · hst-sim/src/score.rs · flow.rs — also show the original's replay marker during post-point replays (from B107, dropped 2026-10-21) BLOCKED: shelved by the user 2026-10-21: replays later
- [ ] **P6** Serve details, lets → `serve.rs` · `play.rs` serve_turn · `ball.rs` (bounce_turn) | t: serve.rs — user 2026-10-16: get a let serve by driving the real game with the virtual pad (tools/vpad.py, as P11k4/k5 do), recording with PINE; vary the server's character, serve power/aim and position until a serve clips the net cord and lands in the box. Then port and verify the let path against that recording. Time limit: stop after 2 hours of trying for a let; if none, write what was tried in the journal and re-block with that — user 2026-10-16: don't play rallies out: save a state just before the serve (scratch slot 8/9) and reload it after each serve that isn't a let, then serve again with a new character/power/aim/position — user 2026-10-16: Carol probably can't hit a let; serve with other characters — user 2026-10-16: try the expert characters for the let
- [ ] **M133** **Debug clip bind** (user 2026-10-19, dev builds only): while a match plays keep a ring of snapshots (one a second, last 60, `play/snapshot.rs`) plus every tick's pads; a bind (F9) writes the oldest snapshot + the inputs since to `context/clips/<time>.clip`. A test helper loads a clip and replays it headless frame by frame (per-tick `state_hash` checks it replays the same), so a bug report like the CPU partner not hitting an easy ball can be replayed with probes attached. Not in release builds. → `play/snapshot.rs` · `play/tick_input.rs` · `play.rs`
- [ ] **P126** Practice-mode exceptions (T8; only the one-player ball machine of M2l reaches them): practice's side pick between aim candidates and its red-marker timeout, the shot effects' practice-only exception, the swing's practice exception (solo with player 1 hitting). Port them for the lobby ball machine → crates/hst-play/src/play.rs (marks.red) · crates/hst-sim/src/shot.rs · swing.rs | j: 2026-10-21-repo-audit (T8 ledger)

### Lowest priority (user 2026-10-07: creatures, NPCs, ball-hit obstacles after everything else)

Done tasks: one line each in `plan/REFERENCE.md` *Done*, full text in `plan/DONE.md`.

### Polish (at the end, user 2026-10-08)

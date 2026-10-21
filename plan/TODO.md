# TODO — open tasks, in order

Rules: `PLAN.md`. Finished tasks move to `plan/DONE.md` (`tools/ctx.py tick <ID>`).

`→` files to open (play.rs fns by name) · `t:` tests · `j:` journal under `research/journal/`. Tests: hst-sim,
hst-data and hst-oracle in the crate's `tests/`; hst-base, hst-play and hst in-module (`#[cfg(test)]`), no `tests/` dir. Fixtures in `context/fixtures/` (`match_s05.bin` slot-5 bot match, `round1.bin` doubles,
`new_recording.bin` dives/smashes/collisions).

### Next (user priorities, in order)

- [ ] **B623** **Mobile auto aspect: full screen + safe area** (user 2026-10-21): on phones with a Dynamic Island / notch and home indicator the Auto aspect ratio looks bad. Make Auto fill the whole screen by default (the game and menus draw edge to edge, under the island/home bar), and on Auto apply the platform safe-area insets to the HUD only (score, pop-ups, legend pinned inside the safe rect) so nothing gets cut off. Menus must still work fine (nothing important under the island/home bar). Check iOS and Android. → `hst-base/src/widescreen.rs` · `hst-base/src/ios.rs` · `hst-base/src/android.rs` · `hst-base/src/graphics.rs`
- [ ] **B622** **No camera switch while movement is locked** (user 2026-10-21): ignore the camera-change input while the players can't move (post-point, cut-aways, serve setup, change of ends, replays); it only works during live play. Presentation only: must not touch the sim or match timing. → `play.rs` (input, camera)
- [ ] **B621** **Second camera accuracy** (user 2026-10-21): the second (alternate) match camera, in singles and doubles: check it against the original (PCSX2 side by side, same moments) and fix what differs; it also doesn't play the post-point cut-away angles; it should, like the default camera. → `hst-sim/src/camera.rs` · `hst-sim/src/cutaway.rs` · `play.rs` (camera)
- [ ] **B620** **Steam Deck 1280x800 layout** (user 2026-10-21): at the Steam Deck's 16:10 resolution text and menus draw wrong. Check menus, HUD and pop-ups with `--shot` at 1280x800 (and other 16:10 sizes), fix what's misplaced/clipped/stretched; layout stays uniform-scaled for any aspect. → `hst-base/src/widescreen.rs` · `play/main_menu.rs` · `play/word_label.rs`
- [ ] **B619** **Emote sound cut at post-point** (user 2026-10-21): when a point ends and the post-point animations start while a player is emoting, stop that player's emote sound so it doesn't play over the reaction. → `play.rs` (react/next_point, emotes) · `hst-base/src/audio.rs`
- [ ] **M145** **Installed mods filter** (user 2026-10-21): Mods → Installed gets a filter (by kind: characters / courts / umpires / texture packs, plus a name search) so a long installed list is quick to narrow; pad and touch, any aspect ratio. → `play/main_menu/mods_screen/grid.rs` · `play/main_menu/mods_screen/packs.rs`
- [ ] **P134** Court prop motions in the match: `hst_sim::court::moving` gives the carts' `Motions` (P133, journal 2026-10-21-p133-prop-motions); the match keeps them per tick (snapshot/rollback/state hash), runs `Motions::pass` once per displayed court frame (intro, cut-aways and replays count; not while paused or on slow-motion in-between frames; `net_hide::buried` hides) and sweeps the ball against the stepped wheel nodes (the collision `World` sits in a `Const` today); the replay fixtures need the cart clock at their start → crates/hst-play/src/play.rs world · play/court_load.rs · play/snapshot.rs
- [~] **B559** Aloha Beach foliage explodes in the browser (user report, Brave, 2026-10-17): after a few rounds the plant/tree meshes' triangles stretch across the screen (static meshes, so their GPU vertex data got stomped; screenshot context/bugs/aloha-foliage-brave.png). Suspects: vertex_patch.rs writes (costume sway) landing in a stale/reused slab range after rig despawn or slab growth (third_party/bevy_render slab_allocator.rs reallocate_slab), or WebGL2's large-slab path. Repro: web build (and HST_WEBGL2=1 desktop) bot matches on Aloha over several games/rematches; skip the sway's vertex_patch::send to bisect; check WebGPU vs WebGL2 → crates/hst-base/src/vertex_patch.rs · noise.rs · third_party/bevy_render/src/slab_allocator.rs · crates/hst-play/src/stage.rs BLOCKED: no repro on Linux/ANGLE-Vulkan (WebGL2 + WebGPU full matches clean); needs the reporter's OS/GPU/Brave version, WebGPU vs WebGL2 and a console log, or a Windows Brave (ANGLE D3D11) run with a human in doubles
- [~] **B555** Player drawn wrong on Zen browser (user 2026-10-17, web build, Firefox-based): 3P (the cowboy) shows a second racket lying loose on the court beside him and a faint ghost racket at his hand (context/reports/2026-10-17/2.png). Looks like a racket/attachment bone matrix or skinning going wrong on Firefox's WebGPU/WebGL; check with Firefox, compare to Chrome on the same frame (crates/hst-base/src/character.rs, crates/hst-base/src/gs.rs) BLOCKED: not reproduced natively or in headless Zen with the report's court and lineup; needs the user's windowed Zen in an online 4-player match (does it persist, does a local match show it) — see journal 2026-10-17-b555-zen-racket
- [~] **B554** Black box on the court on Microsoft Edge (user 2026-10-17, web build, online doubles on court 4 — room court index 3): a solid black rectangle stands behind the net, centre court, with the net's glass-like panel beside it (context/reports/2026-10-17/1.png). Probably a prop or a transparent/shadow material whose texture or blend failed on Edge's WebGPU/WebGL backend. Reproduce (Edge or Chromium with the same backend, that court), find the mesh (cull/lod/gs/shade), fix it (crates/hst-base/src/gs.rs, crates/hst-base/src/textures.rs, crates/hst-play/src/stage.rs) BLOCKED: no repro: native court 4 has no such object and the web harness can't import the ISO on the new landing page; needs a working Chromium/Edge run or more detail from the user (offline? ?webgl2? from the first point or after the change of ends?)
1949cf8c6b5b557f/bevy_render-0.19.1/src/render_phase/mod.rs:1241:40:
index out of bounds: the len is 0 but the index is 0"
- [ ] **B618** Release build warnings (seen in tools/release.sh 1.3.0): (1) crates/hst/Cargo.toml's [[example]] hst and the bin both use src/main.rs — 'found in multiple build targets', every platform; (2) third_party/gilrs-core: unused #[macro_use] in lib.rs:1 and unused strong/weak/min_duration in platform/default/ff.rs:16 (iOS/Android builds); (3) crates/hst/src/update.rs:633 Due::Ask/Install never constructed on one release target (gate them by cfg like their users); (4) crates/hst/android/.../HstActivity.java uses a deprecated API (build with -Xlint:deprecation, replace it). Fix all so release.sh builds clean; apksigner's JDK native-access WARNING is the SDK's, ignore. Also: the web build broke on Settings' named fields (fixed in 3f01349a) — have release.sh/done.sh check the web target so that can't slip again. Before the next release.

### Netplay and mobile (user 2026-10-09: right after the bugs/priority list, in this order)

### Match basics

### Shots

### Players


### AI

- [~] **P11** Opponent AI → `play.rs` bot/bot_serve/intercept/draw_due BLOCKED: umbrella (user 2026-10-21): the open AI work is P11k6, P11l6 and the review P116; everything the bot-only recordings reach is ported and bit-exact — P116 review (2026-10-21): table in journal 26-P116-REVIEW; gaps P127 (change of ends draws), P128 (app step replay), P129 (strike/new-path), P130 (serve hand-over order/aim frame)

### Match

### Presentation
- [ ] **B617** Skybox check, stock and modded courts: some skies show texture seams that don't line up and their animation jumps (user 2026-10-21). Go through every stock court and the court mods, screenshot the sky against the original (tools/screenshot.sh vs --shot) where stock, find where the seams/jump come from (UV wrap, face order, scroll/rotation clock wrapping or resetting) and fix; looks-right only, no bit-exact proof → crates/hst-play/src/stage.rs, crates/hst-base/src/court_mod.rs, crates/hst-base/src/mods_court.rs


### Confirmations


### Stretch

- [~] **B72** Post-point replays still bad and blue screens still happen (bug, user 2026-10-08; moved to stretch 2026-10-12: replays later; B22/B25 didn't cover it all): after some points the replay/cut-away is wrong and the screen goes plain blue (nothing drawn). Reproduce (bot matches over many points, all courts, log the cut-away chosen and when the frame is empty), find each cause and match the original's replay → `play/cutaway.rs` · `play.rs` | t: cutaway.rs — From dropped B127 (user 2026-10-12, dropped 2026-10-16, may already be fixed): after a game-winning point, partway through the replay the score went back to deuce and the match carried on as if the game was never won (the replay counted as a point?). Replays are off in the app until replays are fixed (instant_replay.rs REPLAYS, on in tests); when turning them back on, check the replay never touches the score (re-run scoring/point-end hook, stale snapshot restore, or replay sim state leaking into the live match) and add a test: a game-winning point + its replay keeps the game won → play.rs react/next_point · hst-sim/src/score.rs · flow.rs — also show the original's replay marker during post-point replays (from B107, dropped 2026-10-21) BLOCKED: shelved by the user 2026-10-21: replays later
- [ ] **P6** Serve details, lets → `serve.rs` · `play.rs` serve_turn · `ball.rs` (bounce_turn) | t: serve.rs — user 2026-10-16: get a let serve by driving the real game with the virtual pad (tools/vpad.py, as P11k4/k5 do), recording with PINE; vary the server's character, serve power/aim and position until a serve clips the net cord and lands in the box. Then port and verify the let path against that recording. Time limit: stop after 2 hours of trying for a let; if none, write what was tried in the journal and re-block with that — user 2026-10-16: don't play rallies out: save a state just before the serve (scratch slot 8/9) and reload it after each serve that isn't a let, then serve again with a new character/power/aim/position — user 2026-10-16: Carol probably can't hit a let; serve with other characters — user 2026-10-16: try the expert characters for the let
- [ ] **M133** **Debug clip bind** (user 2026-10-19, dev builds only): while a match plays keep a ring of snapshots (one a second, last 60, `play/snapshot.rs`) plus every tick's pads; a bind (F9) writes the oldest snapshot + the inputs since to `context/clips/<time>.clip`. A test helper loads a clip and replays it headless frame by frame (per-tick `state_hash` checks it replays the same), so a bug report like the CPU partner not hitting an easy ball can be replayed with probes attached. Not in release builds. → `play/snapshot.rs` · `play/tick_input.rs` · `play.rs`
- [ ] **P126** Practice-mode exceptions (T8; only the one-player ball machine of M2l reaches them): practice's side pick between aim candidates and its red-marker timeout, the shot effects' practice-only exception, the swing's practice exception (solo with player 1 hitting). Port them for the lobby ball machine → crates/hst-play/src/play.rs (marks.red) · crates/hst-sim/src/shot.rs · swing.rs | j: 2026-10-21-repo-audit (T8 ledger)

### Lowest priority (user 2026-10-07: creatures, NPCs, ball-hit obstacles after everything else)

Done tasks: one line each in `plan/REFERENCE.md` *Done*, full text in `plan/DONE.md`.

### Polish (at the end, user 2026-10-08)

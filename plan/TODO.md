# TODO — open tasks, in order

Rules: `PLAN.md`. Finished tasks move to `plan/DONE.md` (`tools/ctx.py tick <ID>`).

`→` files to open (play.rs fns by name) · `t:` tests · `j:` journal under `research/journal/`. Tests: hst-sim,
hst-data and hst-oracle in the crate's `tests/`; hst-base, hst-play and hst in-module (`#[cfg(test)]`), no `tests/` dir. Fixtures in `fixtures/` (`match_s05.bin` slot-5 bot match, `round1.bin` doubles,
`new_recording.bin` dives/smashes/collisions).

### Next (user priorities, in order)

- [ ] **B662** **Touch controls without PlayStation buttons** (user 2026-10-23): the on-screen pad (web app and native iOS/Android) labels its buttons ✕ ○ □ △ / L1 R1 / SELECT START. Give the buttons plain names for what they do instead of PlayStation glyphs: in a match Shot / Slice / Lob (□ hidden or blank while it does nothing, B655/B72), Swap hand, Pause, the shoulders as their emotes; in menus OK / Back (and □ for what that screen uses it for, e.g. Delete), relabelled live as the screen changes. The match legend and menu hints shown after a touch name the same words, not ✕ ○ glyphs (legend.rs keys_style picks SONY art for touch today). Bindings, seats and the sim input are unchanged; any aspect ratio, portrait and the two-player phone layout (B118) included. Done when a phone in the browser and the native build show no PlayStation symbols on the touch pad in menus or a match → web/touch.js · crates/hst-play/src/play/touch_pad.rs · crates/hst-play/src/play/legend.rs
- [~] **B654** **Net hides in post-point cut-aways** (user 2026-10-22): when a post-point celebration cut-away has the camera close to the net, the net should not draw (`net_hide.rs` has the original's rule: 2 m to the net plane during a cut-away, 0.8 m otherwise) but it still shows. Find why the cut-away case never fires (cut-away flag not reaching it, the eye/heading it tests, k from the cut-away's fov, or B634/B638's split-view rework) and fix; check with `--shot` on a cut-away that frames a player at the net. → `crates/hst-play/src/play/net_hide.rs` · `play/cutaway.rs` · `play.rs` (camera) BLOCKED: the original draws the net in the net-post cut-away (game byte, slot 5); user decides: keep as original, or hide it within 2 m in cut-aways as our own addition
- [~] **B620** **Steam Deck 1280x800 layout** (user 2026-10-21): at the Steam Deck's 16:10 resolution text and menus draw wrong. Check menus, HUD and pop-ups with `--shot` at 1280x800 (and other 16:10 sizes), fix what's misplaced/clipped/stretched; layout stays uniform-scaled for any aspect. → `hst-base/src/widescreen.rs` · `play/main_menu.rs` · `play/word_label.rs` BLOCKED: can't reproduce at 1280x800 (shots, scale 1-2.25, gamescope): needs a Deck screenshot of the wrong screen + Game/Desktop mode, build, Display settings — 2026-10-21 (interactive): main/settings/chars/confirm rescale right fresh at 640x400/1280x800/1920x1200 and after a live 1280x800->640x400 resize. Fixed: desktop HUD density cap (soft text at scale 2.25). Still need a Deck screenshot if it's something else.
- [~] **B559** Aloha Beach foliage explodes in the browser (user report, Brave, 2026-10-17): after a few rounds the plant/tree meshes' triangles stretch across the screen (static meshes, so their GPU vertex data got stomped; screenshot context/bugs/aloha-foliage-brave.png). Suspects: vertex_patch.rs writes (costume sway) landing in a stale/reused slab range after rig despawn or slab growth (third_party/bevy_render slab_allocator.rs reallocate_slab), or WebGL2's large-slab path. Repro: web build (and HST_WEBGL2=1 desktop) bot matches on Aloha over several games/rematches; skip the sway's vertex_patch::send to bisect; check WebGPU vs WebGL2 → crates/hst-base/src/vertex_patch.rs · noise.rs · third_party/bevy_render/src/slab_allocator.rs · crates/hst-play/src/stage.rs BLOCKED: no repro on Linux/ANGLE-Vulkan (WebGL2 + WebGPU full matches clean); needs the reporter's OS/GPU/Brave version, WebGPU vs WebGL2 and a console log, or a Windows Brave (ANGLE D3D11) run with a human in doubles
- [~] **B555** Player drawn wrong on Zen browser (user 2026-10-17, web build, Firefox-based): 3P (the cowboy) shows a second racket lying loose on the court beside him and a faint ghost racket at his hand (context/reports/2026-10-17/2.png). Looks like a racket/attachment bone matrix or skinning going wrong on Firefox's WebGPU/WebGL; check with Firefox, compare to Chrome on the same frame (crates/hst-base/src/character.rs, crates/hst-base/src/gs.rs) BLOCKED: not reproduced natively or in headless Zen with the report's court and lineup; needs the user's windowed Zen in an online 4-player match (does it persist, does a local match show it) — see journal 2026-10-17-b555-zen-racket
- [~] **B554** Black box on the court on Microsoft Edge (user 2026-10-17, web build, online doubles on court 4 — room court index 3): a solid black rectangle stands behind the net, centre court, with the net's glass-like panel beside it (context/reports/2026-10-17/1.png). Probably a prop or a transparent/shadow material whose texture or blend failed on Edge's WebGPU/WebGL backend. Reproduce (Edge or Chromium with the same backend, that court), find the mesh (cull/lod/gs/shade), fix it (crates/hst-base/src/gs.rs, crates/hst-base/src/textures.rs, crates/hst-play/src/stage.rs) BLOCKED: no repro: native court 4 has no such object and the web harness can't import the ISO on the new landing page; needs a working Chromium/Edge run or more detail from the user (offline? ?webgl2? from the first point or after the change of ends?)
1949cf8c6b5b557f/bevy_render-0.19.1/src/render_phase/mod.rs:1241:40:
index out of bounds: the len is 0 but the index is 0"

### Netplay and mobile (user 2026-10-09: right after the bugs/priority list, in this order)

### Match basics

### Shots

### Players


### AI

- [~] **P11** Opponent AI → `play.rs` bot/bot_serve/intercept/draw_due BLOCKED: umbrella (user 2026-10-21): the open AI work is P11k6, P11l6 and the review P116; everything the bot-only recordings reach is ported and bit-exact — P116 review (2026-10-21): table in journal 26-P116-REVIEW; gaps P127 (change of ends draws), P128 (app step replay), P129 (strike/new-path), P130 (serve hand-over order/aim frame)

### Match

### Presentation


### Confirmations


### Stretch

- [~] **B72** Post-point replays still bad and blue screens still happen (bug, user 2026-10-08; moved to stretch 2026-10-12: replays later; B22/B25 didn't cover it all): after some points the replay/cut-away is wrong and the screen goes plain blue (nothing drawn). Reproduce (bot matches over many points, all courts, log the cut-away chosen and when the frame is empty), find each cause and match the original's replay → `play/cutaway.rs` · `play.rs` | t: cutaway.rs — From dropped B127 (user 2026-10-12, dropped 2026-10-16, may already be fixed): after a game-winning point, partway through the replay the score went back to deuce and the match carried on as if the game was never won (the replay counted as a point?). Replays are off in the app until replays are fixed (instant_replay.rs REPLAYS, on in tests); when turning them back on, check the replay never touches the score (re-run scoring/point-end hook, stale snapshot restore, or replay sim state leaking into the live match) and add a test: a game-winning point + its replay keeps the game won → play.rs react/next_point · hst-sim/src/score.rs · flow.rs — also show the original's replay marker during post-point replays (from B107, dropped 2026-10-21) BLOCKED: shelved by the user 2026-10-21: replays later — user 2026-10-22: when replays land, □ (pad West / Xbox X / Space) asks for the post-point replay again, the controls page names it so, and the on-screen pad shows □ after the point (hidden during serve and rally); B655 took replay off □ for now
- [ ] **P6** Serve details, lets → `serve.rs` · `play.rs` serve_turn · `ball.rs` (bounce_turn) | t: serve.rs — user 2026-10-16: get a let serve by driving the real game with the virtual pad (tools/vpad.py, as P11k4/k5 do), recording with PINE; vary the server's character, serve power/aim and position until a serve clips the net cord and lands in the box. Then port and verify the let path against that recording. Time limit: stop after 2 hours of trying for a let; if none, write what was tried in the journal and re-block with that — user 2026-10-16: don't play rallies out: save a state just before the serve (scratch slot 8/9) and reload it after each serve that isn't a let, then serve again with a new character/power/aim/position — user 2026-10-16: Carol probably can't hit a let; serve with other characters — user 2026-10-16: try the expert characters for the let
- [ ] **M133** **Debug clip bind** (user 2026-10-19, dev builds only): while a match plays keep a ring of snapshots (one a second, last 60, `play/snapshot.rs`) plus every tick's pads; a bind (F9) writes the oldest snapshot + the inputs since to `context/clips/<time>.clip`. A test helper loads a clip and replays it headless frame by frame (per-tick `state_hash` checks it replays the same), so a bug report like the CPU partner not hitting an easy ball can be replayed with probes attached. Not in release builds. → `play/snapshot.rs` · `play/tick_input.rs` · `play.rs`
- [ ] **P126** Practice-mode exceptions (T8; only the one-player ball machine of M2l reaches them): practice's side pick between aim candidates and its red-marker timeout, the shot effects' practice-only exception, the swing's practice exception (solo with player 1 hitting). Port them for the lobby ball machine → crates/hst-play/src/play.rs (marks.red) · crates/hst-sim/src/shot.rs · swing.rs | j: 2026-10-21-repo-audit (T8 ledger)

### Lowest priority (user 2026-10-07: creatures, NPCs, ball-hit obstacles after everything else)

Done tasks: one line each in `plan/REFERENCE.md` *Done*, full text in `plan/DONE.md`.

### Polish (at the end, user 2026-10-08)

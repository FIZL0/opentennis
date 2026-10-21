# Repo audit — 2026-10-21

Read-only sweep of the crates, tools/research, build/CI/vendoring and plan/docs. Nothing was changed. Line numbers
are as of `88882cc5`. Ranked within each section; the top of the file is what to fix first.

## Summary

The core is in good shape: no Ghidra names or retail addresses in shipped `src/`, no std transcendentals in the sim,
every vendored patch is live and licensed, fuzz targets compile against current APIs, mod zip/glTF/updater input is
well guarded, and removed features (Controller Vibration, AO/Bloom/HDR/Soft/Smooth, splash bake) left no residue.

What needs attention:

1. **Netplay trust hole** — the host relays a guest's `State`/`Lobby`/`Pads` to everyone else unchecked.
2. **No correctness gate in CI** — every push to main ships a public beta; tests, clippy and fmt never run there.
3. **User state lives in the CWD / next to the ISO** — including the Hugging Face token.
4. **~25 dead dependencies** left in `hst` (and 5 in `hst-base`) after the P60/P61 crate splits.
5. **Silent-pass tests** — ~208 tests `return` green when the ISO/fixture is missing.
6. **Plan/journal drift** — 5 blocked tasks whose blocker is done, 18 orphan plan files, 16 stale `-READY` journals.

---

## 1. Security and robustness

| # | Where | Finding | Fix |
|---|---|---|---|
| 1.1 | `crates/hst-base/src/net.rs:2409-2411`, `:766-770` | Host relays every in-match `Msg` from a guest to all other peers (only `Away`/`Restart` held back), and `Event::Match` doesn't carry the sender. A doubles guest can push a full `State` into other guests (`hst-play/src/play/resync.rs:60` only checks `np.me != "host"`), forge `Lobby`, or set any seat's pads (`netplay.rs:102-111`). | Carry the sender in `Event::Match`; relay `State`/`Lobby`/`Restart` only from the host; drop `Pads` for seats the sender doesn't hold. |
| 1.2 | `hst-play/src/play/resync.rs:61`; `netplay.rs` pending map | `resync.pieces` grows forever while `last=false` keeps coming; `slot(tick)` accepts any future tick. Memory exhaustion by a peer. | Cap piece bytes and the tick window. |
| 1.3 | `crates/hst-base/src/handshake.rs:73` | `Handshake::check` (M2e) is only called from its own tests; the `Handshake` resource inserted at `crates/hst/src/main.rs:725` is never read; `gameplay_hash` is test-only. Only build IDs are compared, so a disc-pack/rules/mod-gameplay mismatch is caught only later by the desync hash. Its `#[allow(dead_code)] // the transport (M2f) calls it` is false. | Wire it into the join path, or delete it. |
| 1.4 | `crates/hst-base/src/modstore.rs:38,1117,1140-1145` | `hf-token.txt` written CWD-relative with default perms, then chmod 0600 by a second relative call (race + may hit a different file if CWD changes). Doc says "beside settings.txt", but settings live beside the ISO. Android: `external_data_path`. Web: plain localStorage. | Write via `OpenOptions` with `mode(0o600)` at create, into the same dir as settings (or the OS keyring). |
| 1.5 | `crates/hst-data/src/iso.rs:159-161` | Dir records are length-checked, but `rec[32]`, `rec[33..33+name_len]` and `rec[25]` are not: a record < 34 bytes or an oversized name length panics. `walk` also allocates whatever size the disc claims (line 151). First parser to see the user's ISO. Not fuzzed. | Bounds-check; add `iso`, `mor`, `snd`, `layout`, `noi`, `texhash` to `fuzz/fuzz_targets/disc.rs`. |
| 1.6 | `crates/hst-sim/src/court.rs:80-110` | Library code with `.expect("court archive on disc")` + ~9 `unwrap`s, called at match setup (`hst-play/src/play.rs:1146`, `net_hide.rs:195`). A bad court (or court mod) crashes mid-load. | Return `Result`. Same for `sound.rs:427` (`expect("a shout has keys")`, reachable from mod sound banks). |
| 1.7 | `web.rs:238-258` | Counting-allocator `unsafe` has no SAFETY comments (`ios.rs` 23 unsafe / 8 comments, `android.rs` 14 / 9). | Add them. |

## 2. User files in the wrong place

- `store::settings()` (`crates/hst-base/src/lib.rs:65`) puts `settings.txt`, `controls.txt`, `online.txt`, `hints.txt`,
  `names.txt`, `favorites.txt`, `pipelines.json`, `cache/`, `mods/` **next to the ISO** (or `.`). An ISO on read-only
  media or in Downloads scatters/loses state; moving the ISO loses settings, favourites and mods.
- `hf-token.txt`, `mods-store.json` (`modstore.rs:29`) and `mods-wishlist.txt` (`main_menu/mods_screen/grid.rs:614`)
  are **CWD-relative**, so a desktop-launched build writes them into `$HOME` or `/`. Tests also drop
  `mods-store.json` into `crates/hst` and `crates/hst-base`.
- `favourites.txt` in the repo root is an orphan: B394 renamed it, `934b1c6f` dropped the fallback. Players who skipped
  the build with the fallback silently lost favourites. Safe to delete locally.
- `new_switch_runner.txt` is a hand-written note, not game state.

**Suggested task:** one per-user data dir (XDG / `%APPDATA%` / Application Support), migrate from the old location on
first run.

## 3. CI and build

| # | Finding |
|---|---|
| 3.1 | `.github/workflows/pages.yml` is the only workflow: builds wasm + Linux/Windows release on push to main and **publishes a beta**. No PR trigger, no `cargo test` (not even ISO-free sim/data tests), no clippy, no `cargo fmt --check`, no fuzz, no macOS/iOS/Android build. Broken main goes straight to beta.opentennis.dev. |
| 3.2 | No `deny.toml`/cargo-audit over 769 locked packages; no `rust-toolchain.toml`/`rust-version`, so a new stable can break CI with no code change. |
| 3.3 | Actions pinned to major tags, not SHAs; `steamrt/sniper/sdk:latest`; `curl \| sh` rustup; binaryen downloaded without checksum. |
| 3.4 | Release profile has no `lto`/`codegen-units = 1` (optional; thin LTO would shrink shipped builds). |
| 3.5 | `fuzz/Cargo.toml` duplicates all 6 `[patch]` entries by hand — in sync today, nothing keeps it so. |
| 3.6 | `web/apple-touch-icon*.png` are tracked *and* gitignored (`git ls-files -ci`); `tools/web.sh:23` regenerates them. Untrack. |

### Vendored crates (`third_party/`)

All six (bevy_render, bevy_pbr, wgpu, winit, ggrs, gilrs-core) are applied (no `[[patch.unused]]`), versions match the
lock, licences kept, and none can be dropped yet (wgpu 30.0.1 still has the B328 problem). Gaps:

- Undocumented in root `Cargo.toml`: gilrs-core's whole iOS backend (M91); bevy_render `view/mod.rs` sRGB
  `view_formats` change (B438).
- No per-crate record of upstream base, files changed, or upstreaming status; `HST` markers incomplete (winit changes
  4 files / 1 marker, bevy_pbr 2 / 1). Rebasing on Bevy 0.20 will be painful. Add `third_party/README.md`.
- Stale comment paths: winit M101 note says `hst/src/android_pad.rs` (now `hst-base`); bevy_render P47 says hst's
  `mode.rs` (now `hst-play/src/mode.rs:175`).
- `Cargo.lock` ×5 and `Cargo.toml.orig` ×6 inside `third_party/` are never read. Harmless.

## 4. Left behind / unused

### Dependencies

- **`crates/hst/Cargo.toml`** — copied wholesale before the P60/P61 splits; unused in `crates/hst/src`: ggrs, bincode,
  gltf, serde (only serde_json used), wgpu-types, tiny-skia, ab_glyph, iroh-tickets, data-encoding, miniz_oxide,
  iroh-mainline-address-lookup, n0-mainline, iroh-mdns-address-lookup, rfd, wasm-bindgen-futures; all iOS deps
  (libc, objc2, objc2-metal, block2, objc2-quartz-core, objc2-foundation, wgpu, raw-window-handle); all Android deps
  (jni, jni22, libc, webpki-roots, rustls-platform-verifier). Keep `tracing` (`ship` feature). Comments still point at
  old file locations. Check feature unification before deleting.
- **`crates/hst-base/Cargo.toml`** — ggrs, bincode, tiny-skia, ab_glyph, minisign-verify unused there.
- **`crates/hst-oracle/Cargo.toml`** — `hst-sim` used only by `tests/`: move to `[dev-dependencies]`.

### Dead code (zero references in crates/, fuzz/, tools/)

- `hst-sim/src/sound.rs:643,645,647` `CHANGE_ENDS_CALL`, `TIEBREAK_CALL`, `MATCH_CALL`; `sound.rs:21` `JINGLES` —
  **confirm those umpire calls actually play** (probably moved to `umpire.rs`).
- `hst-data/src/exe.rs:175` `Game::tornado_floor`, `:204` `Game::stick_reach` — GAME.BIN tuning read but unused:
  either hard-coded elsewhere (faithfulness gap) or the feature is missing. **Worth checking.**
- `hst-sim/src/ai.rs:565` `Guess::target_x` (and `doubles_ai.rs:217` may recompute it).
- `hst-data/src/texhash.rs:465` `from_pack`, `xb.rs:100` `name_hash`, `npc.rs:892` `call::PAUSE_OVER`,
  `hst-oracle` re-exports `DONE`, `Hook`.
- Test-only pub items: `pose::Nodes::from_parts`, `shade::pass_texture`, `receiver_tile`, `receiver_vertex` → `#[cfg(test)]`/`testing`.

### Stale allows

- `hst-play/src/play/snapshot.rs:12-13` whole-file `allow(dead_code)` "only the test calls them yet" — false now
  (recover/netplay/heard/tick_cost use it); it hides real dead code.
- `hst-base/src/net.rs:34` whole-file dead_code/unused on wasm (4,780 lines).
- `hst-base/src/audio.rs:110` `SoundBank::wavs` allow — now used.

### Untracked strays in main

`crates/hst-data/tests/tmp_b510.rs` (asserts nothing, builds as an integration test) and `research/b510_chair_watch.py`
— B510 WIP; belongs on `wip/B510`, not in main.

## 5. Tests

- **~208 tests skip-as-pass** on missing ISO/fixtures (`let … else { return }` / `eprintln!("skip")`), e.g.
  `hst-data/tests/effects.rs:8`, `hst-oracle/tests/serve_ik.rs:91…`, `hst-sim/tests/ai_rally.rs:495`. A fresh clone or CI
  is green while checking almost nothing. Add one `require_fixtures()` helper that fails when `HST_REQUIRE_DISC=1`.
  The ISO path string is hard-coded 85× across 40 files — the same helper fixes that.
- `hst-sim/tests/ai_rally.rs:494` `receive_matches_the_game` never checks its own result (`_long` at :504 does).
- `hst-sim/tests/cutaway.rs:84` `debug_frame`: `#[ignore]` with no how-to-run line, asserts nothing → delete or move to research.

## 6. Port faithfulness (PLAN rules)

- **Clean-room:** `hst-sim/src/shade.rs:586` `inverse` is "the 16 cofactors in the game's op order (generated from its
  code)" — transcription, and pointless since rendering only has to look right. `ai.rs:515,629`
  `allow(too_many_arguments)` "one argument each as the game passes them" keeps the original's call shape.
- **Unproven ports:** `swing.rs:325` "as decompiled; no recorded case", `npc.rs:1862` "unrecorded" → gap tasks or forced-case tests.
- **Guess:** `npc.rs:1945` "(bearing, volume); a guess from their ranges" → per memory, every guess becomes a task.
- **Sim RNG from presentation:** `hst-play/src/play/reaction_voices.rs:45` draws from `g.rng.shared` (the AI's
  generator) behind an approximate view gate (`ponytail:` at :37). If the gate differs from the original, draw counts
  drift. Reseeded each point, so damage is bounded — still verify.
- **`ponytail:` deferrals:** 184 in the Bevy crates (28 in `hst-play/src/play.rs`, e.g. :715 base shot tables only,
  :3556 stand-in AI loop, :3819 "app's random numbers stand in for the game's"), ~90 in hst-sim. Run
  `/ponytail-debt` and make sure each gameplay one has a plan task.
- f64 math in `foot.rs:364`, `rng.rs:173,386`, `npc.rs:497`, `player.rs:44`, and `to_radians/degrees` in camera/cutaway
  (round-to-nearest vs EE chop) — deterministic across platforms; confirm they match the game where they feed the sim.

## 7. Code quality

- **Giant functions:** `online_step` 789 lines (`main_menu/online.rs:989`), `host` 622 (`net.rs:1719`), menu `step`
  557 (`main_menu.rs:1280`), `setup` 381 (`play.rs:1258`), npcs `step` 367, `load` 345 (`main.rs:615`), npc `callback`
  349 (`npc.rs:1640`), hst-gltf `export` 277.
- **Giant files:** `main_menu.rs` 5,116 · `net.rs` 4,780 · `online.rs` 4,402 · `play.rs` 4,402 · `mods_screen.rs` 3,511 ·
  `mods.rs` 2,812 · `npc.rs` 2,176 · `rally.rs` 2,066. 267 lines over 200 chars.
- **`Settings` is an 11-field positional tuple** with `[bool; 9]` indexed by constants (`main_menu.rs:229`); removing
  one setting touched ~10 test constructors. Named fields.
- **Small duplicates:** `fn hex` ×2 (`frame.rs:17`, `roster.rs:56`); `fn now()` ×3; tokio runtime built by hand in 8
  places and a fresh OS thread + runtime per mod-store call (`modstore.rs:204`); `cross`/`rot_y`/`rot_x`/`mul`/`dot` ×2-3
  in hst-sim (some differ on purpose for madd rounding — merge only true duplicates); `i32_at`/`f32_at` in both
  `mdl.rs` and `replay.rs`.
- **Loading screen** (`crates/hst/src/loading.rs:190-193`) is a 4:3 box at 100% height with `max_width: 100%` — may
  squash in portrait instead of going through `widescreen.rs`.

## 8. tools/ and research/

- **`research/dev-stats.json`** (612 KB, 13 commits of churn): Claude token ledger with local session paths. Move to
  `context/` or gitignore.
- **Hardcoded `/home/ryha`:** `tools/single.sh:65` (use `$(dirname "$0")/pgrep`), `tools/record_npc.py:25`,
  `research/{b31e_glare_poke,b74b_hunt,p3f5_flare_hunt,p14c3f_court8,p3e7_start}.py`.
- **~158 task-named probes** at the top of `research/` (`b510_*.py` style) — effectively all for tasks already in
  DONE.md. Policy: on tick, move `research/<id>_*.py` into that task's journal `scripts/` (`done.sh` can do it). Keep
  the top level for reusable tools (`fn.sh`, `xref.py`, …).
- 7 research scripts referenced nowhere (`hud_rec.py`, `spark_rec.py`, `research/tools/{bounce_model,mdlvif,mdlwalk,mtlwalk,vifwalk}.py`);
  6 research/tools pairs overlapping the `tools/record_*` recorders; 14 `tools/web_*.mjs` single-bug probes known only to their journals.
- `research/tools/fixture_shot_tables.py:5` calls `tools/traj_inverse2.py` (it's in `research/tools/`).
- **Copy-paste helpers:** MIPS stub assembler in 11 files, fixture `frames()/regions()` reader in 14, f32/u32 bitcasts
  in 46, `sys.path.insert` for pine in 156 → one `tools/hook.py`.
- **Shell:** 16/33 scripts lack `set -e`. Unbounded waits: `tools/check.sh:13` (forever `sleep 5` for a slot — combines
  badly with the known sccache-inherited flock), `tools/pcsx2.sh` (`flock` without `-w`), `tools/single.sh:56`.
- No decompiled C, images, binaries or secrets tracked. `research/p7c_tparam_map.py` hardcodes Ghidra names as
  patterns (parses the dump; fine but notable).

## 9. Plan, journal and docs drift

**Counts:** 16 open · 11 blocked/`[~]` · 1,050 done · 6 HUMAN items.

- **Blocked on finished work — unblock:** P35 (P36–P48 all done), P11k6 + P11l6 (P11n1 done; dependency text also
  duplicated), P17p1 (P0b4d done), P11 (only k6/l6 left), P16 (`[~]`, no reason, its subtasks done).
- **Broken TODO lines:** `plan/TODO.md:15-16` is a stray panic-message fragment from B395. Z3 isn't in `**ID**`
  format so `ctx.py` can't see it. M136 still open although merged (overlaps B601). B107 duplicates B72. M7a and P23
  are referenced but exist in neither list.
- **Tasks contradicting user decisions:** P17p1 wants debris/step sound "bit for bit" against PINE; P16 wants cameras
  verified with recorders — both are presentation (feel-right only).
- **18 orphan plan files** match no task: N1, N1d, N1d3, N3, N3d1–4, P0b, P0b4, P0c, P10, P11k, P11l, P17, P23, P7, P8
  (some still say `[ ]`/`[~]` inside). 132 plan files for done tasks have stale checkboxes.
- **Journals:** 16 still `-READY` for ticked tasks (p13-umpire, m2-netcode, m2j-lan, p27-4player, b244, b118, b256,
  b272, b273, b274, b305, b310, m20-chat, p30, p33, b515). 113 dirs have no state suffix, 280 no README, 1 loose file
  (`research/journal/2026-10-08-b63.md`), 4 with uppercase IDs. Either make bare `README.md` mean FINAL in AGENT.md or
  have `done.sh` rename.
- **HUMAN.md:** "who of two teammates gets the ball (N5a)" looks answered by P11n2; P11i, P27, P13 items belong to ticked tasks.
- **Code map:** PLAN lists `tools/overnight.sh` (gone; `single.sh`/`parallel.sh`). Unmapped: 23/45 hst-sim modules
  (`npc`, `rally`, `ai`, `aim`, `sound`, `umpire`, `weather`, …), 16 hst-base modules (`vertex_patch` — the B559 suspect,
  `chat`, `disc`, `jobs`, …), 5 hst-data modules + 4 bins. AGENT.md's `crates/game-sim`/`crates/kit-oracle` are in the
  kit repos, not here — say so.
- **Contradictions:** PLAN "Find it in the decompile, port it" vs the clean-room two-session rule; AGENT.md's
  "`xref.py --similar` … Rust to copy the shape of" vs "write it fresh". TODO header says tests live in the crate's
  `tests/` vs PLAN's "no tests/ dir" for hst/hst-play. Pointers to "README *Character mods*" (it's in
  `docs/DEVELOPING.md`) and "README *Steam*" (doesn't exist). REFERENCE still documents the removed B524 backdrop bake
  and has two duplicated paragraphs (court mods M8f6, web M9). `docs/DEVELOPING.md` lists 3 of 7 crates.

---

## Suggested order

1. Netplay sender check + resync caps (1.1, 1.2) and wire or delete the handshake (1.3).
2. Token write at 0600 in the settings dir (1.4); per-user data dir task (§2).
3. A PR/push CI job: `cargo test -p hst-sim -p hst-data` (ISO-free), `clippy`, `fmt --check`, `cargo deny`; pin the toolchain.
4. Drop the dead deps in `hst`/`hst-base`; fix `hst-oracle` dev-dep.
5. ISO/court parser hardening + fuzz the remaining disc parsers.
6. `require_fixtures()` helper so skipped tests can fail on demand.
7. Plan sweep via `ctx.py unblock`/`tick`/`block`; delete orphan plan files; rename stale journals; fix code map.
8. Check the unused umpire calls and the unused GAME.BIN tuning values — possible real gameplay gaps.

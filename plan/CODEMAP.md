# Code map

Where things live, one section per crate: `agent-runner plan <crate>` (e.g. `agent-runner plan hst-base`) prints one,
`agent-runner plan "code map"` the lot. Update the line for a module you add or move.



## hst-data

`crates/hst-data` — disc formats (no game logic)

- `iso.rs` ISO9660 reader · `xb.rs` XB archives · `tim2.rs` textures · `mdl.rs` models (+collision) · `mtl.rs`
  materials · `ani.rs` ANI2 motions · `layout.rs` court layout/plant records · `exe.rs` GAME.BIN data offsets
  (the only place with retail offsets) · `mor.rs` face morph/UV tracks · `noi.rs` costume noise deformers · `snd.rs`
  sound banks, SPU pitch · `texhash.rs` PCSX2 texture-replacement names · `uselog.rs` `HST_ASSET_LOG` asset-use log ·
  `fixture.rs` test-only access to the disc and `fixtures/` (never `context/`) · `bin/` xbdump, tm2png, mdlstat, bankwav, noidump, texkeys, texnames
- tests: `characters.rs`, `collision.rs`, `effects.rs`, `sound.rs`, `npcs.rs`, `texnames.rs`, … (`tests/`)


## hst-gltf

`crates/hst-gltf` — exports HST's own characters (all motions, faces) to `.glb` as modding reference rigs (`hst-gltf hst fixtures/xb DIR`); other games' extraction and mod packaging live in ../opentennis-mods


## docs

`docs/DEVELOPING.md` *Character mods* — links the mods repo (../opentennis-mods: standard, tutorial, extract/rerig tools); `crates/hst/hst_skeleton.json` is the game's copy of its skeleton


## hst-sim

`crates/hst-sim` — the game, bit-exact, no Bevy

- arithmetic: `ps2.rs` EE FPU · `vu0.rs` VU0 · `libm.rs` game sinf/acosf/atan2f (all three re-export our
  `ps2-float` crate, ../ps2-float) · `quat.rs` matrix↔quat, slerp · `rng.rs` the match's four generators on
  ps2-float's newlib `rand()` + MT19937
- ball: `ball.rs` Flight, bounces, materials, COURTS · `contact.rs` plane sweep · `mesh.rs` triangle sweep, world
  query · `world.rs` prop placement, 20 m grid · `court.rs` court collision world from disc
- shots: `shot.rs` TRAJ tables, launch (`shot/powered.rs`: a mod's `power` tables) · `params.rs` per-character shot records · `swing.rs` contact search,
  timing grades · `serve.rs` walk/toss/serve contact/box aim
- players: `player.rs` locomotion, mover, turn, stamina · `motion.rs` motion state machine · `pose.rs` first-frame
  bone matrices
- match: `score.rs` score/rotation/ends · `judge.rs` line calls, verdicts (Rally) · `flow.rs` post-point timeline,
  serve placement
- camera: `camera.rs` match camera mode 0 · `cutaway.rs` post-point cut-aways, intro, replay picks · `instant.rs`
  instant-replay decision
- AI: `ai.rs` AIParam rows · `aim.rs` CPU shot aim · `rally.rs` doubles AI routines (receive, net, phase) ·
  `position.rs` doubles formation · `path.rs` predicted ball path the CPUs copy
- court life: `npc.rs` background figures · `umpire.rs` chair umpire · `weather.rs` weather/wind schedule ·
  `clouds.rs` · `shade.rs` sun-shade map · `lines.rs` court lines · `court_anim.rs` hole UV/alpha animation
- effects/sound: `effect.rs` effect models · `tornado.rs` wind tornado · `foot.rs` footstep dust · `bodyhit.rs` ball
  hits a player · `surprise.rs` pop-ups · `sound.rs` positional sound, reaction voices · `face.rs` face tracks ·
  `noise.rs` costume sway · `stats.rs` match statistics
- `replay.rs` fixture reader · `bin/replay.rs` replay CLI · `bin/state_hash.rs` per-tick hash, same on x86_64/aarch64/wasm32 (`tests/platforms.rs`); transcendentals only via `libm` (game's) or `musl`, never std
- tests: `ball_path flights live world` (ball) · `shot_tables swing serve` · `player motion` · `score replay` ·
  `camera cutaway` · `line_calls shot_params`


## hst-oracle

`crates/hst-oracle` — test-only function oracle: `hst.rs` (disc check, `HstEe::game`/`from_ram`, `addr`) on our
`ps2-oracle` interpreter (../ps2-oracle); tests compare ports to the game's own functions


## hst-base

`crates/hst-base` — the Bevy app's lower layer (P60: its own crate, so an edit in the match or menus rebuilds
and relinks only `hst`); `hst`'s `main.rs` re-exports it (`pub use hst_base::*`), so `crate::audio` etc. still resolve
there. It never names `hst`: hooks the top fills in go through statics/generics (`graphics::SAVE`, `graphics::plugin::<Game>`)

- `lib.rs` `Args`, `Orbit`, `store` (settings file I/O), `Wire` (snapshot encoding) · `handshake.rs` online match seed
  (`MatchSeed`) and peer handshake check (M2e) · `mods_online.rs` mods in online lobbies: gameplay wire, looks packs,
  stand-ins (M2i) · `mods_umpire.rs` umpire mods: manifest, calls over the donor's bank (M8f3) · `mods_textures.rs` texture packs: manifest,
  order/off (M5a) · `mods.rs`/`mods/`
  `modfs.rs` `modstore.rs` `mods_court.rs` `court_mod.rs` character/court mods, zips, store · `character.rs` disc
  characters, rig, animate · `net.rs`/`net/` iroh transport, lobbies, codes, `net/player.rs` own player id (M197) · `audio.rs` · `graphics.rs` settings,
  auto-tune · `gs.rs` `shade.rs` `shadow.rs` `textures.rs` `lod.rs` `cull.rs` rendering · `widescreen.rs` HUD
  pinning (B17) · `seats.rs` pads → seats · `ui_pool.rs` overlay quad pools only while shown (P39) · `ui_census.rs`
  `HST_UI_CENSUS=1` UI node counts/changes per root · `pipelines.rs` render pipelines recorded to `pipelines.json`,
  pre-warmed from the menu (P55; the dedup is in third_party/bevy_render's `pipeline_cache.rs`) · `web.rs` `ios.rs` `android.rs` platform glue
  (`android_pad.rs` pads) · `disc.rs` ISO import → disc pack · `jobs.rs` work off the frame (B409) · `chat.rs` lobby/match
  chat (M20) · `device.rs` platform + last input · `vertex_patch.rs` per-frame vertex writes without touching the mesh asset
  (B357) · `effects.rs` hit effects · `court_anim.rs` `weather.rs` `noise.rs` sim parts on court · `spring.rs` mod spring
  bones · `occlusion.rs` Character Shading (baked vertex AO) · `inspect.rs` 3D character preview · `hud_gamma.rs` GS-style HUD blend · `mips.rs` menu art mips ·
  `asset_use.rs` drawn-texture log (Z2) · `scene_census.rs` `stall.rs` `HST_SCENE_CENSUS`/`HST_FPS` diagnostics
- test helpers `hst`'s tests use are `#[cfg(any(test, feature = "testing"))]` (hst's dev-dependency turns `testing` on)


## hst-play

`crates/hst-play` — the match (P61: its own crate under the menus, so a menu edit rebuilds only `hst` and a
match edit is checked with hst-play's own tests); `hst`'s `main.rs` re-exports it (`pub use hst_play::*`) and splices
the front-end into its `play` module (`mod play { pub use hst_play::play::*; pub mod main_menu; pub mod menu; }`), so
`crate::play::…` paths still resolve there. It never names the menus: the pause state and preview faces come through
`play::front_end::FrontEnd`, a resource `play/menu.rs` fills in

- `lib.rs` · `play.rs` the match: setup, input, strike/serve/locomote/advance_stroke, bot, simulate, react/next_point,
  camera, draw, motions, balloons, hud · `play/` match parts, netplay (+ `OnlineNotice`, `CHOICES`)/rollback/snapshot,
  `word_label.rs` word.tm2 labels (`FONT`), `legend.rs` the key legend's types and drawing, `front_end.rs` the hooks
  above · `play/fifty_hz.rs` the European disc at 50 Hz: motions/paths retimed to `Build::motion_rate`, umpire
  voice ticks, the 50 Hz bot-match test (`EU_DISC` lock for European-disc tests) · `play/dive_ik.rs` a contact dive's arm
  solve (`pose::DiveIk`) into `Player::arm`/`racket` · `play/save_game.rs` match saves (M193): setup + point-start
  state (`PointStart`, `capture`/`resume`), `Saves` (data dir `saves/`, one per line-up+mode, newest 5), `Head::check` · `mode.rs` mode switches/teardown · `preload.rs` · `stage.rs` court models/layout/sky (from main.rs)


## hst

`crates/hst` — the game on top: menus, modes, startup

- `main.rs` startup, disc load, auto_shot, wasm entry `run` · `play/main_menu.rs` + `play/main_menu/` the front-end
  (menu rows of the legend in `main_menu/legend.rs`; Mods → Installed → Texture packs' list in `main_menu/mods_screen/packs.rs`) · `play/menu.rs` the in-match pause screen (embeds the options
  pages) · `loading.rs` · `sandbox.rs` · `mash.rs` · `update.rs` self-updater
- no `tests/` dir on purpose (in either crate): any file there makes `cargo test -p <crate>` build the whole game a
  second time (as a binary for it); sim-only tests go in hst-sim


## tools

`tools/` — `pcsx2-hst.sh` launch · `pine.py` RAM/states · `vpad.py` virtual pad · `screenshot.sh` ·
`record_p2m2.py` `record_live.py` `record_camera.py` `record_cutaway.py` `trace_live.py` `trace_shots.py`
recorders; unattended runs are `agent-runner run` (`runner.toml`). `research/` — RE scripts (`fn.sh`, ghidra_scripts), journal;
`research/README.md` rebuilds `context/`.

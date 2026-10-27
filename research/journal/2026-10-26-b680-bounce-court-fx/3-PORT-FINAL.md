# B680 port: bounce court effects (live play)

Ported from SPEC.md (decompile locked): the live-play parts of §2, §3.1, §3.4–3.6. Replay bursts stay with B72.

## What landed
- `hst-data::exe`: `Game::bounce_debris()` reads the bounce debris table (count + clod row, count + blade row,
  sink) from GAME.BIN; `tests/effects.rs` checks every value.
- `hst-sim::effect`:
  - `Bounce` takes the wet flag and body-hit flag:
    - wet puffs (0.25 / 0.2, rising 0.01 a frame, `run/spray` texture);
    - a wet smash landing makes an ordinary 0.5 puff instead of the dust cloud;
    - on a wet court bounce, the `run/spray` model replaces the ring;
    - the ring is 1.5× after a body hit.
  - `BounceBits` does the first-bounce trigger (kind stored), the smash burst (30 clods, 120 `rng.sound` draws) and
    the smash pool's aging.
- `hst-base::effects::BallBounce` loads `run/spray` and the `par/tuti` clods, and draws the spray, the wet puffs and
  the clods as facing quads in the court's clod colour, with ground shadows.
- `hst-play/src/play/bounce_debris.rs` contains two systems:
  - point set-up, wet and body hit before `start_effects`;
  - the throw, after `start_effects` and after the footstep effects, so `rng.sound` is spent in the game's order:
    feet, hit sparks, then debris.

## Verification
- Recorded a live smash landing from slot 5 on court 10 (scratch state 8 at vsync 32560; the first bounce is at
  32652). The recording covers the ball, the `rng.sound` generator, the first contact records and the game's smash
  clod pool each frame, vsync 32600..32740 (`fixtures/b680/smash_c10.bin`).
- The game's pool is 30 entries of 0x90 bytes, starting 0x20 into the pool. The pool is owned by the effects manager
  child that was created when the match started.
- `crates/hst-sim/tests/bounce_debris.rs` replays `BounceBits` over the recording:
  - exactly one throw, of 120 draws, leaving the generator in the game's state;
  - every clod's alive flag, position, speed, size and cell bit-exact on every frame of the burst's life (32652
    until all land at 32683) and after.
- By eye: the original at vsync 32653/32658/32666 (`context/b680/shots/orig_*.png`) against the remaster with
  the burst forced on every first bounce (a local debug edit, not committed; `context/b680/shots/dry_*.png`).
  Brown clods fly up and out from the landing and fall back, with dark shadows, in both.
- A rainy court (`HST_WEATHER=3`, `context/b680/shots/wet_*.png`): no ring; a pale spray model and sparkly spray puffs
  at the bounce.
- `HST_NETPLAY=1 tools/check.sh -p hst-play netplay`: 35 pass. `a_mod_plays_alike_with_or_without_its_looks` fails
  because a mod in the user's live mods folder (`oob_pc12_anya`) is over the 64 MB limit. That comes from the mods
  folder, not from this change.

## Not verified / not 1:1
- Sparks and the debris drawing on the same frame: the recorded smash frame had no hit-spark re-roll, so their
  relative order is set from the spec (sparks first) and not proven by a recording.
- The port's footstep effects tick after `start_effects`, so on a frame where hit sparks and a live dive ring both
  draw from `rng.sound`, the port draws sparks then feet, where the game draws feet then sparks. This order was
  there before this task. The debris throw now runs after both, as in the game.
- Wet visuals (spray model, spray puffs, wet smash puff) were only checked by eye in the remaster. There is no rainy
  save state of the original to compare against side by side.
- Smash landings on hard (2.5× scale) and clay courts were not screenshotted. The size scale is covered by the spec
  and the unit tests, and only court 10 (grass) was recorded.
- The crater is lit only when not wet: a rendering detail, not ported (the crater draw is unchanged).
- The replay bursts (clods, blades, the replay size cut) are deferred to B72.

## 2026-10-27 rerun
The port (1fb79cbd) is already merged into main. `tools/check.sh -p hst-base oob_faces_all` still fails: the live
mods folder's oob_pc18_toro still has no face clips. That has nothing to do with B680. It's filed as B684. Rerun
`tools/done.sh B680 "…"` once B684 lands.
The full suite also fails `hst` `plate_emote::tests::a_mods_emote_voices_take_turns` (live mods folder:
fore_pc17_tbone motion 0x2e plays keys 1..5, the test expects 1,2,3,0,1). Also not B680; noted on B684.
done.sh runs the suite before `--blocked` too, so this rerun blocked the task with `tools/ctx.py block` and
committed by hand.

## Not verified / not 1:1
As in the port entry above; nothing new this session.

# M169 — EU 50 Hz mode (Match Speed): decompile, classify, split

**Verdict: go.** The European program (Everybody's Tennis, SCES-54535) is the US program retimed for 50 Hz. It
has no rule changes. Every gameplay difference is one of three things:
- a frame count or per-frame amount rescaled,
- the motions' tick rate rescaled when they load,
- the European disc's own retimed data (TRAJ tables, shot parameters).

So a 50 Hz match is the US sim with a speed switch, fed the EU disc's data. It is not a second rules build like
the Japanese one. This session wrote no code. It built the decompile, classified every differing routine and split
the work into tasks (end of this file).

## How it was compared (scratch, in `context/m169/`, not in git)
- **Overlay ELFs:** the EU overlay ELFs were rebuilt from the EU boot program and `ZZBIN/GAME.BIN`, the way
  `research/README.md` does it for the US (`scripts/mkoverlay.py`).
  - The overlay loads at a different base address. The EU program is a relinked build.
- **Ghidra:** a project (`ghidra/eu`) with the same `DumpDecomp`/`DumpXrefs` dumps (`decomp/eu_game.*`).
- **Function match:** M163a1's masked-instruction hash, run against the US game program. Call targets, address
  high halves and memory offsets are masked, so a function whose only change is a constant comes out as a "shape"
  match, and its counterpart is found directly.

  | | exact | constants only | unmatched |
  |---|---|---|---|
  | boot program | 2370 | 39 | 23 |
  | game overlay | 1231 | 114 | 19 |

- **Pairing:** the 195 non-exact US functions were paired with EU ones (`pairs.tsv`): by the unique shape match,
  else by link order from the last exact match.
- **Diffs:** each pair was diffed as normalised disassembly (`diffs/`, `scripts/dis.py`) and as normalised C
  (`scripts/cdiff.sh`). The changed constants were pulled out and tagged by their EU/US ratio (`consts.txt`,
  `scripts/consts.py`, `bigconsts.py`).
- **Result:** every pair, its class and a one-line note are in `classify.tsv` (`scripts/classify.py`).
  - The motion clock and the motion setter are exact matches.
  - So is every shot, flight, bounce and line-call routine that isn't listed in `classify.tsv`.

## What differs (195 routines)
| class | count | what |
|---|---|---|
| timing | 107 (97 in the match program) | frame counts ×5/6 (60→50, 30→25, 1800→1500, 3600→3000, 1440→1200, 120→100), per-frame speeds ×6/5 (run 0.0717024→0.08604288), per-frame² amounts ×36/25 (gravity 0.00272222→0.00392, speed² thresholds), 1/60→1/50 and 1/600→1/500 (wind), km/h ×60→×50 (3600·50/1000), serve motion rate 1/30→1/25 |
| video | 45 | PAL 512 lines (448→512, 224→256, 240→256), GS mode NTSC→PAL, larger frame buffers (moved VRAM bases), frame-pacing counters |
| aspect | 19 | camera/screen aspect 0.7→0.8 (×8/7), one 8/7 constant |
| motion | 15 | the 50 Hz motion switch and the loaders that obey it (below), plus the clip-length nudge |
| region | 6 | more files on the disc (languages), the language setting, voice-bank language pick, speed readout in km/h only (no mph) |
| logic | 2 | a camera object gains an extra −10° condition (presentation); one background-figure formula drops a 0.02 term |

**How the counts were rounded matters.** They weren't all rounded the same way. Examples:
- 5→4 (×0.8), 10→8, 20→16, 7→6, 80→66: rounded down
- 45→37.5, 220→183.333: exact float
- 901→751: 1 + 900·5/6
- 3600 / 0.25 cycles → 3000 / 0.3003
- the CPU serve wait: a random 60..119 → 50..99

So the 50 Hz values can't be derived as "×5/6". Each one is a separate constant, and the spec has to list them all.

### How motions play at 50 Hz (the motion files are the US ones)
- **The rescale:** when a player loads, the EU program turns a "50 Hz" switch on for each motion it loads. The
  motion loaders (skeletal clip, face morph, UV track) then scale the file's ticks-per-frame by 60/50 in integer
  arithmetic: 80→96.
  - So a clip's length in frames is 5/6 of the US length, and a motion played at speed 1 takes the same real time
    as on a US disc.
  - Where the new length isn't a whole number, the clip-length routine nudges it up by ×1.000001. It does that
    only in the EU build, for all clip kinds.
- **Motions kept at US timing:** the switch is off for the player's stroke motions (the 16 swing clips from
  forehand/backhand topspin through the whiffs, plus the receive and the smash) and for the two serve swings (strong
  and weak toss).
  - These are the motions whose speed is computed from frames to contact (8 frames before contact ÷ frames left; the
    serve's rate per frame), and those frame counts are already EU ticks.
  - The serve setup, walk, toss-up, special shots and reactions are rescaled.
  - Of the four placeholder clips, only the ball-reaction one is rescaled.
- **Creature models** (the AZUMA creature motions) load with the switch off. Their own update code is retimed
  instead.
- **Everything else** loaded with the switch at its default (on) is rescaled. That includes court props and effect
  models.
- **The motion clock is unchanged:** each tick it adds the speed its caller passed. The constant-speed callers
  still pass 1.0, and the rescaled length takes care of them.

### Data the 50 Hz mode needs from the EU disc
- **`TRAJ/`, `SAITOU/TRAJ`:** re-solved at 50 Hz (M163a2). These can't be derived from the US tables, so 50 Hz
  needs EU data present.
- **GAME.BIN:** the shot-parameter base (151 floats, ×1.2 of the US values) at the EU file offset. The rest is the
  US data shifted (M174 reads EU offsets for the 60 Hz path; share it).
- **Also differ:** `SAITOU/HSMODE/HSM.XB0` and three court hole files (UV/animation timing, presumably). These
  weren't diffed field by field.
- **Same bytes as the US:** CSVs, motions and models. Their per-frame meaning is rescaled in code (above).

## What the remaster assumes about a 60 Hz tick
For the plumbing task. Paths are as of today.
- **Fixed timestep 60 Hz:**
  - `hst-play/src/mode.rs` (mode teardown re-inserts `Time<Fixed>::from_hz(60)`)
  - `play/intro.rs` (its own 1/60 accumulator, and `set_timestep_hz(60)` at the hand-over)
  - `hst/src/play/menu.rs` (the pause screen restores 60 Hz twice)
  - `hst-base/src/court_anim.rs` (its plugin inserts 60 Hz)
  - `play/replay_pace.rs` (test clock)
  - `play/instant_replay.rs` (skips by whole steps, any rate)
- **Rendering interpolation:** `overstep_fraction` (markers, tornado, rain, the camera). It works at any rate.
  At 50 Hz on a 60/120 Hz display, every moving thing must interpolate, or motion judders. Check the characters,
  ball and camera first.
- **Sim constants that are really "60 frames a second":**
  - km/h ×60: `sound::kmh`, `npc::ball_kmh`, `instant.rs`, `tornado.rs`, `ai.rs`'s reaction-speed test,
    `build.rs`'s Japanese km/h. EU: ×50.
  - `flow.rs` hold seconds ×60
  - `stats.rs` match time `frames / 60` (EU: /50)
  - `ai.rs` serve wait `% 60 + 60`
  - `instant_replay.rs` `LONG_RALLY` 1800 (EU 1500, one of the retimed counts)
  - `hst-data/src/snd.rs` BGM clock `FPS = 60` (EU sound banks differ too, 77 files)
- **Netplay/rollback** counts in ticks:
  - `rollback.rs`: `DELAY` 2, `WINDOW` 80, `SPREAD` 20, `UNACKED` 120
  - `netplay.rs`: `HASH_EVERY` 30, the lag→ms conversion `* 1000 / 60`, the round counters `% 60`
  - `recover::tick(w, 1/60)`

  At 50 Hz each tick is 20 ms, so the window covers more time. Rollback works the same, but both peers must run
  the same speed: the host's.
- **Front-end accumulators** (`delta * 60` in cursor, flame, match stats, menu): menus, not the match. They can
  stay at 60.
- **Tests:** every replay/match test counts 60-tick seconds. 50 Hz tests need their own fixtures from the EU disc.

## Setting (as the task line asks)
- **The setting:** Settings → Gameplay (M167) → Match Speed: 60 Hz (default) / 50 Hz (European original).
- **When it shows:** 50 Hz is offered only when EU data is there, either as the main disc or a sibling (M166). An EU
  disc alone offers 50 Hz only, until M170's close-enough 60 Hz lands.
- **Online:** the host's speed rules the lobby (M171). The handshake must carry it, and both peers need the EU data,
  or the host streams it (M171).

## Split (tasks added to the plan)
- **M169a: spec** (clean-room, child: it needs this classification). It writes the behaviour of every sim-relevant
  row in `classify.tsv` that is marked timing/motion/logic: each 50 Hz value and its rounding, the motion switch and
  its exception list, the clip-length nudge, the km/h factor, wind, gravity, run speed, CPU waits and the serve
  rate. It must also check the boot program's data tables, which the hash doesn't see (the camera tables turned out
  to be scaled in code, not in data).
- **M169a1: port + verify** (child of M169a): implement `Build`-style 50 Hz switches in hst-sim, read the EU disc's
  data, and compare with the EU recordings.
- **M176: tick-rate plumbing + Match Speed setting** (fresh: it needs no decompile). `Time<Fixed>` at 50 or 60 from
  the setting at every place listed above, interpolation checks, netplay carrying the host's speed, and the setting
  UI.
- **M177: EU disc recordings** (fresh): a slot-5-style bot match on SCES-54535 in PCSX2 (PAL, 50 Hz), and a motion
  clock capture, as fixtures for M169a1.

## Not verified / not 1:1
- **Nothing was ported or run.** This was the decompile-and-classify phase. No PCSX2 run of the EU disc, so the
  classification and the motion finding come from the decompile alone.
- **84 of the 107 timing rows aren't sorted into sim vs presentation.** They're tagged by constant ratio only. The
  spec session sorts them as it reads each one.
- **The boot program's data sections weren't compared.** The function hash ignores data, and M163a2 compared only
  GAME.BIN's data area. A retimed table there would be missed; it's on M169a's list.
- **Some files weren't diffed field by field:** `HSM.XB0`, the three court hole files and the 77 sound files.
- **The video/aspect rows aren't specced.** The remaster renders its own way, and PLAN.md says rendering only has to
  look right.

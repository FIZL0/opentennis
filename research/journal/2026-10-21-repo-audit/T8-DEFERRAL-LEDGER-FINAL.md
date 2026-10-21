# T8 — gameplay deferral ledger

The audit (README §5) counted ~270 `ponytail:` markers (266 today) and asked that each one leaving gameplay
inexact have a task. Harvested: every marker in `crates/hst-play/src/play.rs`, `play/doubles_ai.rs`,
`play/npcs.rs` and `crates/hst-sim/src/`, plus a pass over the rest of `crates/hst-play/src/play/` for anything
touching the sim. Each was read in context and sorted into: **stale** (the gap is closed; comment fixed),
**gap** (a task now exists), **AI** (noted on P116, the AI review), **equivalent** (says why the shortcut gives the
game's result, nothing to do), **presentation/perf/tooling** (no task by PLAN's rules).

## Tasks made

| Task | Markers |
|---|---|
| P117 smash/powered aim | play.rs strike (`margins.inside(.., false, .., false, ..)`: smash deep mode, powered shots' unscaled margins) · swing.rs `smash_scatter` (no mode blend, base smash table) |
| P118 serve bend order | play.rs `serve_bent` (kind before the stick turns topspin flat, order unchecked) |
| P119 whiff details | play.rs `whiff` (smash middle = midpoint of top/bottom) + found while reading: its path horizon is `g.reach.grades.len()`, character 0's 28, not the hitter's own count |
| P120 dive frame | play.rs `find_dive` (this frame's run for the previous frame's locomotion state) |
| P121 dive slide early end | swing.rs:325 (as decompiled, no recorded case; the audit's "unproven port") |
| P122 pelvis table | pose.rs `first_frame` (within 1e-4 of the game's table, not bit-exact; the turn uses it) |
| P123 ball at rest | play.rs rally check (no rest detection on steep surfaces, 1800-frame timeout) |
| P124 umpire tick order | play.rs `simulate` (she steps before the phase messages; her line gates the post-point wait) |
| P125 prop node walk | court.rs `nodes` (checked only on court 10, all one-node props) |
| P126 practice exceptions | play.rs red marker/side pick · shot.rs:270 practice-only exception · swing.rs:639 practice exception (only the M2l lobby ball machine reaches them; under Stretch) |

## AI (note added to P116)

P116 already says every AI ponytail becomes a gap task there; the note lists them so it doesn't have to re-harvest:
play.rs `ai_serve_step` (stand-in goal, B27d), the serve follow-through break-off, `ai_wait` (partner's shot record,
`Formation::hold`), `ai_wait_singles` (ALL coin once a point; shot choice 10's dash spot), `bot_serve` and its
contact (aims at the press), `ai_aim` (exhibition level fixed at 3, mark = opponent's spot, reach heights, singles
return of serve, short-ball flag, kind-1 contact), `ai_update` (stand-in loop; mind not remade for a new match);
hst-sim `serve.rs` `ai_search` (vertical speed from next frame's rise), `player.rs` `toward` (modes 0/1 only),
`ai.rs` 631/842/983; `play/mod_match.rs` (mod AI row's costume level). doubles_ai.rs 221/246/315 are already spelled
out in P11k6/P11l6.

## Stale (comment fixed in play.rs)

- `rally_tables` / `rally_spin` "variants come with P3": P3 is done; the variant tables/records are loaded and used
  by `play/timing.rs` (`record_spin`).
- `reach` (the base reach) grades "their source is P3": each player's own replace them in `reach_of`.
- `aim_target` "bots aim with a random stick … P11": bots pass `ai_aim`'s stick (`ai_contact_stick`).
- `react` "the app's random numbers stand in for the game's": the team pick draws `g.rng.shared` (P3d).

## Equivalent / unreachable (nothing to do)

play.rs: humans from the pads at setup (sides never change mid-match). doubles_ai.rs:350 (the serve message is
sent at set-up, where the original clears). hst-sim: judge.rs:63 (every collided surface has a material),
flow.rs:218 (players idle then), mesh.rs:295/488 (out-of-map texel never seen; box culling only skips work),
ball.rs:88 (explains the predictor's net), player.rs:675 (baseline server never reaches a bound), shot.rs:352 (yaw
never past ±π), swing.rs:410 (cap never reached by disc counts), rng.rs:209 (checked against the disc),
court.rs:156 (mod scenery, an addition), hst-data exe.rs:616 (enable byte set for every character). hst-play: timing.rs:49/78 (P3's 211 recorded launches exact),
tick_input.rs:101 (one press per frame, as a pad read).

**Reaction voices' shared draws** (audit §5): `reaction_voices.rs` draws `g.rng.shared` behind a camera view test.
Checked by reading: the shared generator is reseeded from `rand()` (not from itself) at every new point
(`Rngs::new_point_clouds`), and nothing gameplay draws it between the reactions and that reseed, so a view-gate
difference can only change which voice/shout plays. Presentation; no task.

**Court generator order** (npcs.rs 246/426, rain.rs:356): the court generator feeds spectators, gallery, emitters
and creature sounds only; it is reseeded from `rand()` each point and nothing in the ball/player sim reads it.
Presentation.

## Presentation / perf / tooling (no task)

play.rs: held-ball pose mid-crossfade, reaction root without mover collision (post-point; the next serve's
placement resets everyone), unfinished body-hit reaction not snapped, match call/match-over phase (front-end),
jingle and BGM timing. npcs.rs: all 16. hst-sim: effect.rs, foot.rs, shade.rs, sound.rs, camera.rs, cutaway.rs,
bodyhit.rs:208, pose.rs:360, rng.rs:117/136/250, stats.rs:146, npc.rs (all; incl. 1945's guessed bearing/volume,
which is SFX), bin/replay.rs. hst-play/play: popups, replay_camera, replay_pace, instant_replay, flare, tornado,
net_hide, ball_shadow, emote, foot_fx, ceremony (its confetti xorshift only matters to a second match's seed, which
the port's own front-end already makes non-original), match_stats, change_ends, snapshot, recover, netplay.

## Not verified / not 1:1

- Everything in the task table above is a gap by definition; none was checked against the game in this task (that
  is each task's job).
- The "equivalent" calls rest on the markers' own evidence and on reading the code, not on new recordings.
- The reaction-voice and court-generator conclusions were checked by reading where the generators are reseeded
  and drawn, not by a recording.
- Markers outside hst-play/hst-sim (hst-base, hst, hst-data: net, audio, menus, mods, rendering) were not
  harvested one by one: those crates hold no match sim (PLAN code map), and the scan of their marker lists showed
  only presentation, netplay, mods and tooling.

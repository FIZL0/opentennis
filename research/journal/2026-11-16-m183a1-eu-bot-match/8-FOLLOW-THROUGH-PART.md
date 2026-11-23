# M183a1, part 8: slot 5 loads again; a computer player's follow-through; the US match plays to point 6's rally (PART)

No decompile was read. Every finding comes from recordings of slot 5 and from replaying them through the port.

## Slot 5 loads again (PCSX2 v2.9.52 for the slot copies)
- As the user asked (2026-11-22), PCSX2 v2.9.52 is now installed beside the current build, at
  `~/.local/opt/pcsx2-v2.9.52/`: the AppImage plus its extracted `squashfs-root`.
- `tools/pcsx2-hst.sh` starts the slot copies (`HST_PCSX2=n`) with it when it is there. `HST_PCSX2_BIN` overrides the
  binary. The process is still named `pcsx2-qt`, so `stop` and the PID check still work.
- State 5 loads in it ("Loaded state from slot 5"). The `plan/HUMAN.md` item is removed.

## New recordings (git-ignored)
- `fixtures/m183a1/us-rng-long.bin`: slot 5's generators over vsyncs 7520–12719 (`scripts/us_rng_rec.py 5 5200`). It
  is the same as `fixtures/p3b/rng_s05.bin` where the two overlap, apart from three torn samples.
  `the_us_bot_match_plays_as_recorded` reads it when it's there and keeps play's generators for 5150 ticks to compare.
- `fixtures/m183a1/ai-net-long.bin`: every AI routine call over vsyncs 7595–11411 (`tools/record_ai_rally.py`;
  302 calls dropped). Every recorded receive, NET and BASE call replays exactly through `hst_sim::rally`. So the US
  divergence at 3596 came from play's inputs to the routine, not from the routine itself.

## A computer player's follow-through plays on until its own stick breaks it off
- At play tick 3597 (rec vsync 11180), player 3's BASE call chose a different plan in play (kind 4, plan 10) than in
  the game (kind 1, plan 7). I patched the recorded call one input at a time to play's value (the shot records' `n`,
  the stand test's quad, the body's facing). Only the facing flips the result. The game's facing was (+0.707, −0.707);
  play's was (−0.707, −0.707).
- Both bodies start from (0, −1) and turn toward the same run direction, (−0.85, 0.53). The game turns the long way
  round. The turn chooses its direction from the pelvis of the motion playing at the frame's start. In the game
  (`match_s05`, player +0x3df0) that motion is the return stroke, 0x11, still playing 47 ticks after contact. In play
  it was the ready stance (1).
- Why: play's routine preamble called `follow_through(p, true)`, as if the computer player always pressed the stick.
  So the follow-through broke off at recovery + 2 (32 frames) and the player stood. In the game it lasts until the
  routine's own stick (or press) breaks it off, and that frame's run starts from the stroke's motion.
- Fix (`doubles_ai.rs`): the preamble now passes `false`. After the routine, `break_off` ends the follow-through when
  the routine's stick or press counts as input (`follow_over`'s rule). The player runs or stands in that same frame,
  with `body.motion` set to the stroke's motion, so the turn starts from it.
- Result: player 3's facing over 3595–3597 matches the game's (0.383, 0.707, 0.924). Every player matches from the
  first serve to tick 4010, apart from each point-over's last tick (1084, 2354, 3190, 3795). Play's AI generator stays
  on the recording's draws through point 6's serve contact (lost only at the serves' contacts and point-overs: 218,
  1086, 1269, 2356, 2578, 3192, 3484, 3797). Both are asserted.

## Where the US match parts now: point 6's serve is a tick short
- After 30–40 (point 6) and after 40–40 (point 8), the game's serve phase is one tick longer than play's (180 vs 179,
  154 vs 153). The other set-ups (after 0–15, 0–30, 15–30 and 30–30) match.
- The extra tick comes at the phase's start. The recorded ball sits unplaced at (0, −1.064, 0) for two ticks there,
  where it sits for one after the earlier points. From then on the serve and the rally are the same, a tick apart. The
  "players part" at 4011 is just that shift.
- Candidates: a score-dependent call or banner (game point, deuce) that holds the set-up a tick. Telling them apart
  needs the set-up's timing rules from the decompile, so this goes through a spec: B766.

## EU
Unchanged since part 7. Point 2's set-up still runs its new-point AI reset 5 ticks later than play does (89 draws at
recorded 577, all in one tick). The follow-through fix does not move it. Filed as R14 (a per-call AI-draw capture over
EU recorded 570–580, now possible with the v2.9.52 copy).

## Status
M183a1 is blocked on B766 (the serve set-up tick) and R14 (EU point 2's set-up). When both land, carry on from
here: the EU match tick for tick, then the flip and its release note.

## Release note
This branch adds one line for the follow-through fix: computer players in doubles now hold their follow-through
until they run, as in the original, and that changes their first steps after a stroke.

## Not verified / not 1:1
- A dive press during a follow-through ends it without a stand or run that frame. That was already so before, when
  the preamble ended it. No recording covers it.
- Whether the game's follow-through also holds for a computer player outside the doubles routine (`bot()` in
  `play.rs` still passes `true`). It is not on this task's files, and no recording here covers it.
- The serve set-up's extra tick at 30–40 and deuce (above). Its cause is unknown.
- EU point 2's set-up offset (part 7). So `match_speed::FIFTY_HZ_SIM` stays off and there is no release note.
- `ai-net-long.bin` dropped 302 calls. None of them fall at 3590–3600, but the replay can't cover them.

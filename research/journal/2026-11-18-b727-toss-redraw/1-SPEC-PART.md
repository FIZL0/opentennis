# B727 — 1: spec for point 2's toss and serve draws (2026-11-18)

Spec session (clean-room): `SPEC.md` beside this file. The port phase implements it and checks it with
`the_us_bot_match_plays_as_recorded`.

## Found

- **Where the 4 extra draws come from.** I counted the AI draws per player at each hit message in play. At point 2's
  toss, play re-heard point 1's last stroke (player 1's smash). Its opponents, players 0 and 2, drew 16 + 1 each,
  and players 1 and 3 drew 15 each, so 64 in all. The game drew 60 = 4 × 15, the no-team redraw.
- **Why the game hears nobody.** The hit message's team tests read the match's last-hitter record. The next-point
  step resets that record to "nobody" each time the point-over phase is left. The M184 spec said point set-up
  doesn't clear it, and its section 5 (a later toss re-hears the last stroke) followed from that. Both are wrong.
  The record is "nobody" at every toss (SPEC section 1).
- **The next gap, at point 2's serve contact (game 69, play 68).** The doubles AI's set-up keeps the serve history
  on every set-up after the first: it clears it only in its full form, which runs while the match has had no rally
  yet. Play clears it every point (`reset_positions` builds a fresh `Player`). So play's receiver made no serve pace
  draw. The same full/partial split decides the "team hasn't hit yet" mark (play's `first`). SPEC section 2.
- **Experiment, reverted (spec sessions write no Rust).** Toss on the no-team path, plus `ai_serves` kept across
  `reset_positions`:
  - the AI generator matches the recording at every tick it covers (to about 1464, the end of `rng_s05.bin`);
  - the serve contact's two aim draws still come a tick early, as on point 1;
  - players match from 219 to **2354** (before: 1270).
  - With only the toss change: players to 1351, generator one draw short from the contact (1276).
  - Probe: `HST_M183A1_FROM=1100 HST_M183A1_AIDRAWS=1260,1500`.
- **PCSX2 can't load the slot states.** PCSX2 is now v2.9.179, and it refuses slot 5's state (made with v2.9.52),
  so I couldn't read the last-hitter record live at point 2's toss. The recording's draw counts stand in for that
  check. Slots 3 and 4 are presumably affected too, so recapturing needs the user (HUMAN.md item below).

## Port phase: what to change

- `toss_heard`: every toss takes the no-team path. The last-stroke branch goes, and `Game::last_stroke` too if
  nothing else reads it.
- Keep each player's serve history (`ai_serves`) across `reset_positions`. Clear it only on the full-form set-up
  (the match's first serve set-up, with a partner AI).
- `first` (today `!ai_hit`): set by the full form, cleared by every later set-up and by its own team's hit.
- Then raise the test's assertions: AI generator equal through the recording's end, players through 2354 (or
  wherever they part after the port).

## Not verified / not 1:1
- **The last-hitter reset isn't read live.** Slot 5's state won't load in the current PCSX2. The rule comes from the
  decompile, and the recording's draw totals (60 at the toss, 69 at the contact) agree with it.
- **Fault and let points:** that the record is also reset before a second serve's toss isn't checked against a
  recording (none has a fault in the covered span).
- **The switch that forces the partial set-up:** which modes turn it on is unknown. It is off in slot 5.
- **The "team hasn't hit yet" mark's new rule** (cleared at every later set-up) changes nothing in slot 5's covered
  span: every team hit in point 1. It is untested there.
- The held ball's extra tick at 1175 and the phase's one-tick level difference (1262) are as before (M183a1 6-*),
  not part of this task.

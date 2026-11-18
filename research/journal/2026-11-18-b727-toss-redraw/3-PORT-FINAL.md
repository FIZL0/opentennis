# B727 — 3: port (2026-11-18)

Port phase from SPEC.md, decompile locked.

## Changed

- **Every toss is heard as a hit by nobody** (SPEC 1, 5): `toss_heard` always takes the no-team path
  (`doubles_ai::heard_nobody` + `ai_redraw`) for every computer player. The re-heard last stroke (M184 section 5) is
  gone. `Game::last_stroke` stays (the doubles AI still reads it for a real hit).
- **Doubles point set-up, full or partial** (SPEC 2): `doubles_ai::point_setup`, called from the AI's new-point
  reset in `ai_update`, before the timing draw. The full form runs on the match's first set-up (no AI mind yet,
  which is the "no rally yet" mark in play: the mind is made at the first set-up of a match). With a partner AI
  object it clears the serve history and sets "team hasn't hit yet". Without one it does nothing. Every later
  set-up keeps the serve history and clears "team hasn't hit yet" (`ai_hit = true`). `reset_positions` keeps
  `ai_serves` across points in doubles. Singles is unchanged: the spec covers the doubles set-up only.
- The replay switch (SPEC 4) is always off in play, which has no rewound point replays, so nothing models it.

## Verified

`the_us_bot_match_plays_as_recorded` (slot 5, four CPUs):
- every player stands where `match_s05.bin` has them from the first serve to tick **2354** (before: 1270);
- play's AI generator is on the recording's draws at every tick through the end of `rng_s05.bin` (about 1460),
  apart from 218 and 1269 (both serves' aim draws a tick early, a known gap) and 1086 (point 1's point-over, which
  this change doesn't touch). Point 2's toss draws 60 and its contact 69, as recorded. Both are asserted now.
- The other 50 Hz bot-match tests still pass.

## Not verified / not 1:1

- **Fault / let tosses**: that the second serve's toss also hears nobody comes from the spec (read from code). No
  recording with a fault lies in the span `rng_s05.bin` covers. The bot-match test's players check (to 2354) is the
  only coverage, and it stops before `match_s05`'s faults.
- **"Team hasn't hit yet" cleared at later set-ups**: in slot 5 every team hit in point 1, so this rule changes
  nothing in the covered span. It is untested.
- **The full form without a partner AI** (a human beside a human in doubles): it isn't covered by a recording.
- **The replay switch**: it isn't modelled, because play doesn't replay points yet. When point replays land, a
  replayed point must take the partial form.
- Where players part at 2354 and the earlier tick offsets around point 1's end (1085–1090, 1175) and point 2's
  set-up are not part of this task (M183a1 6-*).

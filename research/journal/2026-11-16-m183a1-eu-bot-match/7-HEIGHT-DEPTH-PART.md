# M183a1, part 7: the contact height's sign and the reaction's depth; the US match now plays to point 5's rally (PART)

Both fixes come from recordings (`fixtures/match_s05.bin`, slot 5). No decompile was read.

## The contact height was passed upside down
- US point 3's return landed about 0.1 m off. A new probe (`HST_M183A1_SHOT=from,to` in the US test) puts the
  recorded live ball and each player's aim, swing start and hitter fields beside play's.
- Working back from the aim showed that the stick (16, 55)/127 and the hitter both matched. The game's contact height
  was +0.6577 m (up). Play passed −0.6577 because game space is y-down. So the aim's rising-ball narrowing (×78%, when
  the height is under the rising limit) was applied to a stroke that didn't get it.
- Fix: the rally contact passes `-ball[1]` (`play.rs`, `advance_stroke`). With it the US phases match through point
  4's point-over (12 phases; it was 7).

## The reaction pulls a deep player to 4 m behind the baseline
- A scan of every recorded tick for sudden jumps: in `match_s05` the reaction's tick moves a player from deeper than
  15.885 m (11.885 + 4) to exactly 15.885 four times. The jumps are at 3073, 4900, 7991 and 17987, from 16.18 or from
  the mover's limit of 17.885. x is kept. Players nearer the net are left where they are.
- Fix: `react()` clamps |z| to 15.885 when it stops each player (`reaction_depth`).
- After both fixes, every player matches from the first serve to tick 3596 (point 5's rally), apart from the last tick
  of each point-over (1084, 2354, 3190), where play is a tick behind and has already placed the players. This is now
  asserted in `the_us_bot_match_plays_as_recorded`, with `compare >= 12`.

## Where the US match parts now: 3596
Player 3 (the receiver's side, in the rally) stands still a tick early in play (3596; the game stands at 3604–3605,
then at 3620–3621), then runs a different line. The ball still flies as recorded at that point. `rng_s05.bin` ends at
about tick 1460, so no generator recording covers it. A new recorder, `scripts/us_rng_rec.py`, would take one, but slot
5's state doesn't load in PCSX2 v2.9.179 (`plan/HUMAN.md`).

## EU: point 2's set-up (recorded 164 ticks, play 154)
- New recording: 7000 ticks of the three generators from the EU start state (`eu_rng_rec.py 8 7000`). It is in
  `fixtures/m183a1/eu-rng-long.bin`, and its first 1300 ticks are the same as `fixtures/m169/eu-rng.bin`.
  `the_european_generators_play_as_recorded` reads it when it's there.
- With it, play's AI generator follows the game through recorded 571, the end of point 1's point-over. At point 2's
  set-up the game makes all **89** AI draws in one tick, at recorded 577. Play makes the same 89 (68 + 19 + 2), but
  spread over 572–574, five ticks earlier and over three ticks. So play's new-point AI reset runs at the set-up's start,
  while the EU game runs it 5 ticks in, all at once. The US match has no such offset.
- The two fixes above don't change the EU match. It still parts at point 2's set-up.
- What would separate the candidates (a 50 Hz wait before the new-point reset, a stall, or the order of the set-up's
  steps): a per-call AI-draw capture over EU recorded 570–580, which the EU start state allows. Not done this session.

## Not verified / not 1:1
- **Whole-match proof not done.** The US match parts at 3596 (point 5's rally) and the EU match at point 2's set-up.
  `FIFTY_HZ_SIM` is not flipped, and there is no release note for it.
- **The reaction's depth limit**: seen only as the 15.885 snap on the reaction's tick, four times, all on the far side
  of the baseline. Whether a player inside the court is ever moved is not known: no recording shows it.
- **The contact height's sign elsewhere**: fixed on the rally contact only. The serve and smash take their own aim
  paths and already matched.
- **The US AI generator past about 1460**: there is no recording, and slot 5 can't be loaded to make one (PCSX2
  v2.9.179, `plan/HUMAN.md`).
- **The EU set-up's 5-tick offset**: not explained (see above).
- **The last tick of each point-over** (1084, 2354, 3190): play is a tick ahead there, as before.

# M184 port phase: the first toss announced, the European release a tick later (PART: open questions)

The port was written from `SPEC.md` with the decompile locked. Both first-serve tests now run to the contact, bit for
bit up to it. Three open questions went back to the spec (`SPEC.md` *Open questions*): a later point's toss, the
phase at the swing press, and the served ball's small launch difference.

## What changed (play)
- **Release rule** (`toss_released`): the ball leaves the hand once the posed toss frame (0 on the press tick) plus
  the motion's speed (1) reaches the build's release frame. US: the 45th tick after the press, as before. EU: the
  38th (play released on the 37th).
- **The toss is announced** (`toss_heard`, called after the launch's two shared draws): the hit path
  (`hit_flight`, `since_hit`) starts from the toss. With the last-hitter record at nobody (the match's first point),
  each computer player in order runs the doubles hit message with no hitter (`doubles_ai::heard_nobody`: rally
  state only) and the no-team redraw (`ai_redraw`): the four timing errors, then the picks without the dive chance
  (kept), reaction 0, guess cleared. That is 15 AI draws each, 60 on slot 5. Then the predictor and doubles side's
  hit message (`doubles_ai::heard(HIT)`), as a stroke sends it.
- **The one-tick hold is already in play's tick order.** I first held the ball at the hand point for the release
  tick, as the spec words it. That made the toss samples line up with the live ball, but the serve AI then saw the
  ball one step behind the game's and pressed (and struck) a tick late. Play's controls run before its ball step,
  so its ball step in tick k does what the game's ball update does first in frame k+1. The game's hold is that
  order, and play steps the toss in its release tick as before. Play's ball samples are therefore a tick ahead of
  the recording's once the ball flies. The tests align for this.
- The redraw changed the AI's serve values, which moved two scenario tests' moments: the bots' first fault now
  comes at tick 9497 (`a_faulted_serve_cant_be_returned`, budget 4000 → 12000), and the singles server entries
  need 8000 ticks (`a_server_enters_its_rally_return_roll_first`, was 5000).

## Proof (`play::fifty_hz` tests, both in the suite, about 1 s)
- `serve_to_contact`: the players (on the ground) through the contact; the held ball bit for bit until the
  release; the toss curve bit for bit from the tick after the release to the contact; the served ball moving off
  the curve the tick after the contact, within 2e-4 of the recording.
- US (`the_us_first_serve_set_up_plays_as_recorded`): release 169, contact 219, and `rand`, shared and AI
  generators equal to `rng_s05.bin` at ticks 123/124 (toss press), 168/169 (release: +2 shared, +60 AI),
  214/215 (swing press), 219/220 (contact: +4 shared, +68 AI).
- EU (`the_european_first_serve_set_up_plays_as_recorded`): release 114, contact 141. There is no generator
  recording for this match.
- The test reads the recording's live ball now, not the copy each flight step saves first.
- Full suite 1272 passed. Clippy (hst-play, all targets) clean.

## Not verified / not 1:1
- **A later point's toss isn't announced to the AIs** (open question 1). The spec doesn't say what shot the AIs
  note for it. So the whole-match probes still part at the second toss.
- **The match phase turns to rally at the contact, not on the tick after the swing press** (open question 2): 3
  ticks (US) / 6 (EU) of phase byte difference before each serve's contact.
- **The served ball's launch is 6–9e-5 off** (open question 3), so the first point's rally plays out differently
  later (US: point 1 lasts 378 ticks in play against 720 recorded).
- **The two aim draws come a tick early**: play aims at the countdown's second-last frame (an existing choice, the
  "ponytail" in `bot_serve`). The game's AI generator shows no change between the press (+1) and the contact
  (+68), so they come there. The totals and the values match because nothing draws in between.
- **The server is on the ground at contact** (the game's jumps, y −0.167): an existing choice in `serve_ik_start`.
  The test compares positions on the ground.
- **Order of the 60 draws per player** is checked only as the generator's state after them, which proves the
  count, not the per-player split. The values decide the server's quick serve and serve error, and the press and
  contact land on the recorded ticks.
- **EU generators are unchecked** (no `rng_*` recording for the European match). The European release tick, swing
  press tick and contact tick match.

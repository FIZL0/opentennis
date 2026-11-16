# M184 port phase 2: a later toss re-heard, rally from the press, the served ball exact (FINAL)

Ported SPEC.md sections 4–6 with the decompile locked. Both first-serve tests now pass bit for bit through the
served ball's first three ticks, and the match phase matches every tick through the contact.

## What changed (play)
- **The last-stroke record** (`Game::last_stroke`: hitter, swing branch, lob, launch velocity, where the hitter
  stood). `strike` writes it as it sends the AI hit message. Only a new match clears it, never a new point.
  `toss_heard` now announces a later point's toss as that stroke heard again (`ai_heard_stroke`, the same work as a
  real hit: opponent → serve history and shot memory, reaction and guess; own team → noted, the doubles repick at
  the **stored** position). With no stroke yet (the match's first toss) it does the no-team redraw as before. The
  singles hit message (`doubles_ai::singles_hit`, the dive → dash) reads the record too, not the current last
  hitter and position.
- **Rally from the press**: the serve swing starting, struck or whiffed, sets `rally_due`. `simulate` turns the
  phase at its start (`enter_rally`, which also calls the umpire's rally entry). That comes after every player's
  turn and before the ball step, the step being the game's next frame's ball update. The server still plays its
  serve in that window (`serve_swinging`): the human dispatch, the server's motion choice, the doubles human-record
  skip, and the emote gate keep treating it as serving. The singles wait AI stays off until the contact, so the AIs
  don't change, as the spec says. Everything else that reads the phase (bounce sounds, the flight's rally flag,
  stamina drain, the predictor's calls, the loco phase) now sees rally from press + 1.
- **The serve's aim is built from the server's position on the press tick** (`ServeSwing::from`).
- **The computer server's aim stick is quantised as the pad's** (`ai_stick`: each axis rounded to steps of 1/127).
  This one comes from measurement, not from the spec: aiming from the press tick changed nothing (the server
  doesn't move in either first serve), and the launch was still 6–9e-5 off. Scaling the stick showed the error
  follows the stick component (US depth, EU side). Rounding each axis to n/127 made both served balls exact
  (EU 34/127, US −2/127 and −39/127). Rounding to nearest, not truncating (EU 33.78 → 34). The pad driver's dead
  zone does **not** apply: with it the US stick zeroes and the serve goes elsewhere.

## Proof (`play::fifty_hz`, in the suite)
- `serve_to_contact` now asserts the served ball bit for bit for the three ticks after the contact (it allowed 2e-4
  before), that rally starts on the recording's press + 1, and that play's phase equals the recording's every tick
  through the contact (play a tick ahead, like its ball).
- US: press 215, rally 216, contact 219. EU: press 134, rally 135, contact 141. Generators (US) as before.
- Whole-match probes (ignored, run by hand): US point 1 now lasts 514 ticks (was 378, recorded 720). The rand,
  shared and AI generators equal the recording through the return's contact (recorded tick 298) and part before
  320 → M185. The first serve set-up phase now matches in length in both builds.
- hst-play suite 262 passed. Clippy (hst-play, all targets) clean.

## Not verified / not 1:1
- **A later toss's re-announce isn't checked against a recording**: the probes part (M185) before the second toss,
  so the re-heard stroke's draws, and the doubles repick at the stored position, are untested beyond the suite.
- **The second serve's serve history**: play clears each player's shot memory and serve history at every serve set-up,
  first or second, so after a fault the receivers' history doesn't keep the fault twice, as the spec's quirk says.
  The spec doesn't say when the game's set-up resets fully, so this wasn't changed.
- **The singles dash on a re-heard dive** goes through the AI side's hit (`singles_hit`), which persists. Play's
  own singles positioning (`ai_wait_singles`) starts afresh each point and doesn't see it.
- **Which record the hitter comes from**: play takes it from the last-stroke record. The spec's team test names the
  match's last-hitter record, which the game sets only for a hit in rally. The two differ only for a stroke struck
  after the point is over (a fault receiver's late lock).
- **Aim draw timing**: the spec says the aim's two AI draws come on the press tick. `rng_s05.bin` shows one draw at
  the press (215: the swing kind) and the rest at the contact (219). Play draws the aim one tick before the contact,
  as before (`bot_serve`'s "ponytail"). Nothing draws in between, so the values are the same. Not moved.
- **The human stick for the serve aim** is still read at the contact (the spec has the aim fixed at the press).
  Only the position moved to the press. Not checked: no recording of a human serve with the stick moving during
  the countdown.
- **Rally AI aims aren't quantised**: only the serve's stick goes through `ai_stick`. Whether the game's rally aims
  pass through pad steps too isn't measured (M185 may show it).
- **Whiffs**: from press + 1 the referee and the flight's rally flag see a whiffed toss. Play's own whiff call
  (`serve_whiff`) still runs too. No recording of a whiff was compared.
- **"Serve set-up done" marks, the "no rally since the game began" mark and the phase tick counter** (spec section 6
  steps 1–3) have no counterpart in play that reads them between the press and the contact. Not ported.
- **The server is still on the ground at contact** (the probes' 3-D compare parts at the press on y): an existing choice
  (`serve_ik_start`).
- EU generators are still unchecked (no recording).

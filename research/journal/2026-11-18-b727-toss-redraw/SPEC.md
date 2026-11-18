# B727 spec: what a computer player hears at a later point's toss, and what its point set-up keeps

Covers the 64-vs-60 AI draws at point 2's toss in slot 5's bot match (M183a1 journal, 6-POINT-TWO), and the
69-vs-68 draws at point 2's serve contact that show up once the toss is right. It corrects two parts of the M184
spec (its section 5 and the serve-history line under it).

## Background the port already has

- The toss is announced as a hit (M184): on the release tick every computer player gets the match's hit message
  and runs its hit work, then its redraw (timing errors, picks, doubles extras).
- The "nobody hit" path for that message (M184 section 2): no shot noted, no reaction, pace or guess draws,
  reaction frames 0, guess cleared, dive chance kept, 15 draws per computer player in player order.
- The match's last-hitter record (score globals, P0b) and the next-point step that sets up server, receiver and
  side (`Score::next_point`; play already sets its own last hitter to "nobody" there).
- The AI's point reset with its "match start" net pick (`Mind::reset`).

## 1. The last-hitter record is "nobody" at every toss

- The hit message's two team tests ("the hitter is on my team", "the hitter is on the other team") read **the
  match's last-hitter record**, not the last-stroke record. Both need a hitter of 0 or more (M184 section 2).
- A real stroke sets the record to its hitter (along with the shot count going up).
- **The next-point step sets it to "nobody" (−1)**, and the ball-out side with it. That step runs every time the
  point-over phase is left with the match not over: after a won point, a fault and a let alike. (The same step
  then either moves server/receiver/side on or, for a second serve or a let, keeps them.)
- Nothing else writes the record between that step and the next toss. A re-toss (the ball bounced before the swing
  press) has no stroke in between either.
- **So every toss hears "nobody"**: the first point's, every later point's, a second serve's, and a re-toss. Each
  computer player takes the no-team path described in M184 section 2, exactly as on the match's first toss: 15 AI
  draws each, in player order, on the release tick.
- The last-stroke record is still read when the message arrives, but only the two team branches use it, so at a
  toss it changes nothing. **M184 section 5 doesn't happen**: a later toss doesn't re-hear the previous point's
  last stroke, and after a fault the second serve's toss doesn't re-note the faulted serve. A real hit is
  unchanged: its own stroke sets the record just before its message, so the team branches run as now.

Evidence: `rng_s05.bin` moves the AI generator by exactly 60 on point 2's release tick (recorded vsync 1226), with
four computer players. Play's re-heard last stroke (player 1's smash) drew 64: the smash's opponents, players 0 and
2, each drew one extra timing draw (the fast-ball check, which the opponent path makes) and one reaction draw.
With the toss on the no-team path, play's draws equal the recording through the toss.

## 2. The doubles AI's point set-up: full or partial

Each computer player's point set-up (the serve set-up message) comes in two forms. Which one is decided by the
match's **"no rally yet" mark**:

- the match sets the mark when it starts (as it leaves its intro phase, and again as it leaves the match-over phase
  for the next match);
- it clears the mark on **entering the rally phase** (the tick after any serve's swing press, struck or whiffed,
  M184 section 6).

In a normal match the mark is therefore set only for the match's first serve set-up: every later set-up (a new
point, a second serve after a fault, a let) comes after at least one swing press. One match-wide mode switch, off in
slot 5's match, forces the partial form every time (see Open questions).

**Both forms**, every set-up:

- the last shot and the shot before go to "none" (as play does today);
- then the point's timing draw and count follow, as play already runs them.

**Full form** (mark set, switch off):

- if its partner slot has an AI object (`partner_bot`): the net rate goes to 50, a fresh net pick is drawn at those
  odds (one AI draw), **the serve history is cleared** (the latest serve and the one before both go to "none"), and
  the "team hasn't hit yet" mark is set; a pair of computer partners also settle their formation lean here (as play
  already does at the match's first point);
- if it has no partner AI object, none of those; the "team hasn't hit yet" mark is left as it was.

**Partial form** (every other set-up):

- **the serve history is kept** from the point before;
- the "team hasn't hit yet" mark is **cleared** (set to "the team has hit"), whether or not its team actually hit
  in the points before;
- no net-rate reset, no net pick, no formation lean.

What the two kept/cleared values decide (both are read only by the timing draw):

- **The serve history** feeds the serve's change-of-pace check: on an opponent's serve, a receiver whose serve
  history holds a serve before this one draws the serve pace chance (one AI draw, then the pace itself if it
  passes). With the history kept, from point 2 on each receiver compares the serve with the one its team received
  on the point before (or, after a fault, with the faulted serve).
- **The "team hasn't hit yet" mark** is play's `first` (today `!ai_hit`). While it is set: no fast-ball check
  (no draw) and the stroke error is halved. Its own team's hit clears it (as now). The new parts: only the full form
  sets it, and the partial form clears it at every later set-up. So a team that never hit in point 1 (an ace, a
  double fault) is no longer "first" at point 2.

Evidence: point 2's serve contact in `rng_s05.bin` moves the AI generator by 69 (aim, launch and the hit message's
redraws), where play drew 68. Play clears each player's serve history at every set-up, so its receiver (player 3)
had no serve before and made no serve pace draw. With the history kept across points (first set-up only clears it)
and section 1 in place, play's AI generator matches the recording at every tick it covers (to about tick 1464,
the end of `rng_s05.bin`), with the serve contact's two aim draws a tick early as on point 1, and every player
stands where `match_s05.bin` has them up to tick 2354 (they parted at 1270 before).

## 3. Order within the tick (unchanged)

- The next-point step (record to "nobody") runs as the point-over phase is left, before the serve phase's entry
  sends the serve set-up message to the AIs (where the full/partial set-up runs).
- The toss's hit message, on the release tick, inside the server's tick, after the ball is placed (M184).

## Open questions

- Which match modes turn on the switch that forces the partial form is not identified. It is off in slot 5's
  bot match (the full form ran at its first point: the net pick draw and the serve history clear are both seen).
  Until it is known, the port can treat it as always off.
- That the next-point step also runs after a fault and a let comes from the step itself handling "second serve
  pending" and "let". No fault or let point has been compared against a recording yet.

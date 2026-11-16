# M184 spec: a computer server's serve from the toss to the contact, at 60 and 50 Hz

This covers the four places where play and the original part on slot 5's first serve (M183a1's journal, gaps 1–4).
Applies to every server, singles and doubles, human or computer. The toss rule and the toss-time message reach
every computer player; the swing rules are B700's and stay as they are.

## Background the port already has

- The serve walk/toss/swing states, the toss press (B700), the toss flight (its curve already matches the recording
  bit for bit, only its timing doesn't), the AI's contact pick and countdown press (B700), and the hit-message
  redraw each computer player runs when a ball is hit (timing errors, then picks, plus the doubles extras), as
  play runs it today for real hits.
- The motion clock: each tick a player's motion first poses at its current frame (the "posed frame"), then moves
  its frame on by the motion's speed. A motion started this tick is posed at frame 0 that same tick.

## 1. When the ball leaves the hand (gap 1)

Order inside one player's tick: the controller (AI or pad) fills its buttons; then the serve-state rule runs; then
the motion advances (pose, then step); then the pose/hand update; then any pending release or contact is carried out.

- **The toss press** (in the walk state) starts the toss motion (overhand or underhand clip) from frame 0 at speed
  1 and moves to the toss state. That tick's motion advance poses frame 0. So on the n-th tick after the press, the
  serve rule sees a posed frame of n − 1.
- **The release test**, every tick in the toss state while the ball is still in the hand: release when
  **posed frame + motion speed ≥ threshold**. The threshold is **45** in the US build and **37.5** in the European
  build. In both builds the toss clip runs one frame a tick (speed 1).
  - US: the release comes on the **45th tick after the press**.
  - EU: the release comes on the **38th tick after the press** (37 + 1 is the first sum ≥ 37.5). Play releases on
    the 37th (it compares its own tick count, counting the press tick as 1, against 37.5). This is one tick of the
    3-tick European lag.
- The test only marks the release ("ball up" plus "release pending"). The launch itself happens at the end of the
  same player tick, after the motion advance and the hand update. The ball starts at the toss hand's point.
- **The ball holds at the hand point for one tick.** The toss launch places the ball at the hand point and then
  marks it as flying. The ball's own update runs earlier in the match frame than the players' ticks, and it moves
  only a flying ball. So the ball isn't moved in the frame it is launched: on the release tick it reads the hand
  point. From the next tick on it follows the toss curve from the curve's first step, one step per tick. Its first
  step is a plain flight step: the launch resets the flight's tick count and bounce count to 0, and nothing skips
  or delays that step. Play instead steps the ball on the release tick. Checked in both recordings, on the ball's
  live position: on the release tick it is the hand point, and the next tick it has moved. US: release on recorded
  tick 169, where the shared generator draws 2 and the AI generator draws 60. EU: release on recorded tick 114.
  Ticks are counted from the first serve set-up sample = 0.
- **The second hand-point sample is a recording artefact, not the game.** Each flight step first saves the ball's
  position before it moves it (a "previous position" copy), and M183a1's tick list reads that copy, not the live
  position. The copy lags the live position by exactly one tick all through the flight, which is what doubles the
  hold and adds a tick to every flight comparison made from it. A held ball's copy equals its live position, so
  the lag only appears once the ball flies. The port matches the live position (hand point on the release tick,
  curve sample 1 the next). Any test that keeps reading the copy compares it one tick late.
- While the ball is up: if it bounces before a swing press, the server tosses again (the toss state starts over
  and sends the "toss" message). A swing press goes to the swing, masked by toss kind: an underhand toss takes only
  the lob button, an overhand toss only topspin/slice.

## 2. The toss is announced like a hit (gap 2)

The toss launch is a full ball launch, the same one a stroke uses. Its spin is 0.1, its gravity is the ball's
gravity factor × 0.0027222224, its vertical speed is √(2·g·|hand height − apex height|), its flight time is
vertical speed / g, and its sideways speeds are (apex − hand) / flight time. Like every launch it draws **two values from the shared generator**
(play already draws these). Unlike play, it is launched as a hit that is announced:

- the predicted ball path is replaced by the toss's path (the computer server's contact pick reads this path; it
  has no path to read until now), and
- **the match's "hit" message goes out to every player's AI**, as when a stroke is played, as long as a match is
  on. This happens on the release tick, inside the server's tick, after the ball is placed.

What the hit message does to each computer player on a toss:

- The message's notion of "who hit" is the match's last-hitter record. It is **not** the server: the toss doesn't
  change it. At the match's first point the record says **nobody** (−1, measured live on slot 5: −1 through the
  first toss, set only at the first real contact). On later points it still holds the previous point's last hitter,
  because point set-up doesn't clear it. The shot count is 0 at the toss.
- Each computer player then does exactly its usual hit-message work for that "hitter":
  - Singles: it notes the shot as its opponent's or its own team's, then redraws its timing errors and picks.
  - Doubles: the rally re-entry needs the rally state and a shot count above 1, so it doesn't run at the toss. Then
    the team branch, then the doubles redraw (timing errors, picks and the doubles extras).
- **"Nobody" belongs to no team.** Both team tests ("the hitter is on my team", "the hitter is on the other team")
  first require a hitter of 0 or more. So with the record at −1 both are false. Parity never comes into it. For each
  computer player, singles and doubles alike, that means:
  - no shot is recorded, neither as the opponent's last shot nor as its own team's (the records keep what they
    held), and none of the own-team bookkeeping runs;
  - the redraw takes its "not my team, not the other team" path: the reaction, pace and guess draws (opponent's hit
    only) don't fire. The reaction frames are set to 0 and the guess is cleared. The dive chance (point start or
    own team's hit only) isn't drawn and keeps its old value. The doubles extras that need an opponent's hit don't
    fire either;
  - what is drawn, in this order, is **15 values**: the four timing errors, each a sign coin then a size (stroke,
    volley, smash, serve: 8 draws; the doubles smash size has two forms, beside a human partner or not, and both
    draw once), then 7 picks: quick serve, then two shot-choice chances (body shot, low shot), then the four
    level picks. Doubles and singles draw the same 15 here.
- Slot 5's first toss (four computer players, doubles) therefore draws 4 × 15 = **60 values from the AI
  generator**, in player order, all on the release tick. This matches `rng_s05.bin` exactly: the AI generator
  moves 60 and the shared generator 2 on recorded tick 169, and nothing else on that tick. On a later point the
  record still names the previous point's last hitter. Then the usual team branches run for that player
  (opponent's shot recorded, reaction/pace/guess draws for the other team), exactly as after a real hit by them,
  with the shot count 0.
- **What it decides.** The redraw replaces, for every computer player including the **server**, the timing errors
  and picks drawn at the point's start. The server's serve then uses the toss-time values: its quick-serve pick
  (the contact pick takes a rising ball only on a quick serve), its serve timing error (the countdown press rule
  adds it), and its serve level (which serve, so the swing kind and the aim). The receiver's and partners' rally
  timing comes from these values too.

## 3. The swing and the contact (gap 3, and the 50 Hz strike)

No new rule. The swing follows B700: once the toss path is predicted, each tick the AI picks the best contact frame
on it (falling ball unless quick serve, best height, earliest on a tie). It takes the pick when it is under 9 frames
away, then presses when countdown + serve error < 9, with the swing-kind draw on the press tick. The swing motion and
its contact search follow from the press.

The 60 Hz and 50 Hz contact ticks follow from three inputs, which sections 1 and 2 change: when the ball leaves the
hand, when it starts to move, and which quick-serve pick and serve error the server holds. With the toss-time redraw,
the European release one tick later and the one-tick hold, play's European contact (3 ticks late in M183a1) should
land on the recorded tick. Nothing at 50 Hz is retimed beyond the 37.5 threshold.

The recorded serve ticks, counted from the first serve set-up sample = 0, read from the server's serve sub-state,
its "ball up" flag, the ball's live position and (US) the generators:

| | toss press | release (ball up, hand point) | swing press (ball up cleared) | contact (ball launched) |
|---|---|---|---|---|
| US, slot 5 | 124 (AI draws 1) | 169 (shared 2, AI 60) | 215 (AI draws 1) | 219 (shared 4, AI 68) |
| EU bot match | 76 | 114 | 134 | 141 |

Press to release is 45 ticks in the US build and 38 in the European one, as section 1 says. The port's two
first-serve tests are done when play reaches each of these ticks. M183a1's "tick 143" for the European contact
was read from the ball's previous-position copy and in that test's alignment. The live-position contact is 141.

## 4. The serve launch at 60 Hz (gap 4)

The contact itself is play's (last hitter = server, shot count + 1, aim, launch with its shared draws). The
difference in M183a1 (the recorded ball rises, play's dips, so play's point 1 is a fault) comes from the inputs
going in: play aims with the point-start serve level and serve error, the original with the toss-time ones (section
2). The four shared draws at the contact match once the AI generator is aligned. No change to the serve aim or
launch math.

## Order on the release tick (server's tick)

1. Controller → serve rule: release test passes, ball marked up and release pending.
2. Motion advance, hand update.
3. Toss launch: ball to the hand point, the toss path, two shared draws, the predicted path replaced, the hit
   message to every AI (each runs its redraw, in player order).
4. The ball reads the hand point this tick (its live position). The ball update moves it from the next tick on.

## Open questions

none

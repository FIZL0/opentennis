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
- **The ball doesn't move on its first ticks.** In the recording the ball reads exactly the hand point on the
  release tick and on the tick after it. Only then does it start along the toss curve, at the curve's first step.
  From then on it is play's curve, shifted later by the ticks it held. Play instead steps the ball on the release tick.
  Two things add to the hold: the ball's own update runs earlier in the match frame than the players' ticks, so a
  ball launched during a player's tick isn't moved until the next frame; and the toss launch marks the ball as
  "tossed", and the ball update handles that before the flight steps. The port matches the recorded positions:
  hand point on the release tick and the next one, then the curve from its first sample.
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
- With nobody as the hitter, no opponent's shot is recorded and no reaction or pace draws fire. Slot 5's first
  toss (four computer players, doubles) draws **60 values from the AI generator in total**, all on the release tick.
  With 60 draws there, the AI generator matches the recording through tick 237 (M183a1). Read as the usual redraw,
  that is 15 per player: 8 for the four timing errors (stroke, volley, smash, serve; a sign coin and a size each)
  and 7 picks (quick serve, body shot, low shot, the four levels; no dive chance). The port reuses the redraw it
  already has and checks it against that count.
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
the European release one tick later and the recorded two-tick hold, play's European contact (3 ticks late in M183a1)
should land on the recorded tick. Nothing at 50 Hz is retimed beyond the 37.5 threshold.

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
4. The ball reads the hand point this tick and next. It moves from the tick after that.

## Open questions

- Is the second tick at the hand point part of the game (the "tossed" handling), or a repeated sample in the
  recorder? The flight after it lags play's by the same 2 aligned ticks all the way to the contact, which points
  to the game. The port matches the recording either way.
- The per-player split of the 60 draws (15 each) and the hitter's team when the record says nobody: does "nobody"
  count as team 1 by parity, or as no team? Both are inferred from the total and the usual redraw. The port confirms
  them with the generator recording (`rng_s05.bin`) at the release tick.
- The European press tick wasn't checked against a European generator recording (there isn't one). The 3-tick
  European lag is taken as release +1, hold +1 (as at 60 Hz) and the redraw's change to the contact pick. Section 3
  is only confirmed once the port's European contact lands on recorded tick 143.

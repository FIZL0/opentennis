# B672 — how the walking spectators react to a point (SPEC)

Scope: the walking spectators (the six "walker" figures a court may place, each with its own seven-clip set). The
trigger creatures, court 5's seated crowd and the gallery's sounds already follow their own rules (P14, P3e) and are
not changed here. Everything below is presentation: none of it touches the match's own generators or timing. The
only random draws involved are the court-life generator's (the "shared" generator the walkers already use), and the
spec says exactly when they happen so the existing walker recordings stay exact.

## The clips

A walker's clip set has seven clips, numbered here by their place in the set (0..6, files `_01`..`_07`):

| clip | what it is | loops |
|---|---|---|
| 0 | idle | no (restarts itself) |
| 1 | idle variant | no |
| 2 | the long reaction (~110–120 frames; the body dips and rises a long way: the "excited" move, a jump on some sets) | no |
| 3 | stepping aside | yes |
| 4 | stopped after being bumped | no |
| 5 | the short clap (~10–14 frames) | yes |
| 6 | the match-lost reaction (~55–70 frames) | no |

What is already ported and stays: clips 0–5, the idle loop's 1-in-5 variant, the stagger that restarts the idle loops
after a new point, the match-start clap, the gallery's pick of three clappers, the cheer marks. New here: **when** a
walker reacts at all, the reaction at a set's (and the match's) end with clip 6, the facing that goes with it, and the
second reaction at the match-over message.

## Inputs at the moment a reaction is taken

- the judge's call for the point (plain point 0, out 1, fault 2, double fault 3, let 4, net-out 5, illegal hit 6);
- what the point did to the score: nothing (0), won a game (1), won a set (2) — a set includes the match's last
  set; a fault or let is "no point" (−1);
- whether the match is now over (some team has the sets it needs);
- the winning team; whether the gallery "favours" that team (`sound::favoured`: a team with a human on it, or both
  teams when both or neither have one);
- players on court (2 singles, 4 doubles; the original also has a one-player mode, see below);
- the gallery's pick flags (one per walker: set = "claps").

## 1. When a walker reacts

A walker takes a reaction (its reaction state goes to "point to react to"; its stagger counter goes to "done", −1) on:

- **the point-decided message**, but only when the call is *not* a fault or let **and** either the call is a plain
  point (0) or the point won a game or a set. So a point ended by an out, double fault, net-out or illegal hit gets
  **no walker reaction at all** (no clip change, no turn) unless it also won a game or set. A plain winner always does.
- **the match-over message** (sent once, after the last point's own reaction), unless the call is a fault or let
  (it never is then). The walkers react a second time: the reaction below is taken again from the start.

The gallery's pick of clappers (three draws or more, `npc::cheerers`) still happens at every point-decided message,
whether or not the walkers react — exactly as now. (It is skipped only when the pick was carried over from an
instant replay; see 5.)

## 2. Which reaction (taken on the walker's next tick after it was told)

Checked in this order:

1. **One-player mode** (one player on court): clip 5 (clap) from frame 0, and a cheer mark at its neck (as a picked
   clapper's, delay = slot × 3). No draw. (The remaster has no one-player match; nothing to do unless one is added.)
2. **The point won a set** (result 2):
   - match **not** over → clip 5 from frame 0, with a cheer mark. Every walker, picked or not. No draw.
   - match over and the gallery favours the winner → clip 5 from frame 0, with a cheer mark. No draw.
   - match over and the gallery does **not** favour the winner (a computer team beat the human's team) → clip 6
     from frame 0, no cheer mark. No draw.
3. **Otherwise** (as ported now): picked → clip 5 with a cheer mark; not picked → clip 2; then one draw for a
   random frame in [5·slot, 5·slot + 5).

Then the reaction state becomes "reacted".

Clip 6 is not looped and is not one of the clips that hand back to the idle when they end: it plays once and holds its
last frame until the next new point's stagger restarts the idle loop (like clip 5, which loops until then). Clip 2
hands back to the idle when it ends (as now).

## 3. Which way it turns (the same tick, before the clip is chosen)

A reacting walker turns toward (0, ±6.4) as `npc::walker_facing` does now (+6.4 when the chosen player stands at
z ≥ 0, else −6.4). The player whose end decides it:

- a set won, match not over → the winning team's first player (as now);
- a set won, match over, favoured winner → the winning team's first player;
- a set won, match over, unfavoured winner → **player 0** (the walkers turn to the losing human side's end — player 0
  is on team 0);
- anything else → the winning team's first player (as now).

No turn at all for a point that gets no reaction (1).

## 4. Cheer marks

Each clip-5 reaction above registers a cheer mark at the walker's neck with delay slot × 3, exactly as the picked
clappers' do now (`Cheers::add`, then `follow` while it claps). Clip 6 and clip 2 register none. On court 5 the
point-end rule for its own 93 marks (`court5_again`) is unchanged.

## 5. Kept across an instant replay (presentation detail)

The pick flags are copied aside at every serve set-up out of a replay; when the serve set-up of an instant replay
comes, they are put back and the next point-decided message (the replayed point's) does **not** pick again (no
draws), so the same walkers clap in the replay as in the point. The remaster's instant replay doesn't re-run the
walkers; only needed if it ever does.

## 6. The match-over moment

On the match's last point the walkers react under 2.2 (a set won, match over). The match-over message then makes them
react again by the same rule: they restart their clip from frame 0 (clip 5 → a new mark, clip 6 → plays again). In
the remaster the match-over phase is the result screen's lead-in; take the second reaction where the gallery already
handles the match's end (`Cheers::match_over`).

## Observed in the original (slot 5, court 10, four computer players)

`scripts/b672_watch.py` (logs in `context/b672/`), one whole match to its end:

- every plain-winner point: three walkers clap (clip 5, held until the next serve's stagger), the fourth plays clip 2
  for ~105–115 frames, then idles; the pick flags go back to all set at each serve;
- a net-out that won nothing: the pick changed (draws taken) but no walker reacted — they stayed idle, untouched;
- a net-out that won a game: they reacted as for a plain point (three clap, one clip 2);
- faults: nothing;
- the first set's last point (result 2, match not over): **all four** clap from frame 0, the unpicked one included;
- the match's last point (result 2, four computer players, so the winner is favoured): all four clap; ~110 frames
  later the match-over message: all four react again (clip 5 restarted);
- the same match with player 0 made to count as human from the first set's end on (so the computer team that won
  is unfavoured): at the match's last point all four play clip 6 and turn toward player 0's end instead of the
  winners'; at the match-over message clip 6 again. Details in `1-CAPTURE-FINAL.md`.

## Open questions

none

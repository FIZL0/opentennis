# B674 spec: whose reaction the post-point cut-away shows, with one human

Clean-room spec. It describes the original's behaviour in words; the port session works from this and the
recordings, with the decompile locked.

## Summary

The user asked whether, with a single human on court, the original prefers the human's losing reaction or the
winner's winning one after a point, and asked us to match it.

**The original has no human rule here.** Who the post-point cut-away shows, and whether that player plays a win or
a loss reaction, depend only on who hit last, who hit before them, who won the point, whether the point ended a
game, and how many instant replays the match has shown. The players' controllers (human pad or CPU) are never
read by either decision. A one-human match therefore follows exactly the rule already ported for the all-CPU
recordings (P16a/B22, `hst_sim::cutaway::roles`, `motion::reaction`).

One real gap turned up while checking the rule (see *Gap*): the every-third-replay swap is held back for a player
who was **hit by the ball**, not for a player who is diving.

Controllers matter only in two neighbouring places, both already ported and not part of this rule:
- the instant replay decision (`hst_sim::instant::decide`: a human hitter's strong shots and aces replay; ported,
  oracle-checked);
- while the post-point camera runs, if at least one player on court is human, a pad press lets the human switch or
  skip the camera (a separate camera-mode handler; with every player a CPU it never runs).

## Terms

- **Team:** players 0 and 2 are one team, 1 and 3 the other (team = player index & 1). In singles player 0 is one
  side, player 1 the other.
- **A, B (the hit roles):** two player indices the game keeps through the point.
  - At serve setup: A = the server, B = the receiver.
  - At every hit: A becomes the hitter. B becomes the old B on the point's first hit (the serve), otherwise the
    old A. So during a rally A is the last hitter and B the one before (or, before any return, the receiver).
- **Last hitter:** the player who struck the ball last this point (none = −1, e.g. before the serve).
- **Winner team:** the team awarded the point.
- **Game-ending point:** the point's score event is a game or a set (or match).
- **Replays shown:** the match's count of instant replays shown so far.
- **Hit by the ball:** the player's per-frame state is "hit by the ball" (the states are: standing, running,
  stroking/diving/whiffing, hit by the ball). It is the same condition that gives that player the hit-by-the-ball
  reaction at the point's end.

## Rule 1: who is shown (the shown player and the other subject)

Decided once, when the point is awarded (the score message), only when players on court is not 1 (not practice).
Inputs: A, B, last hitter, winner team, game-ending, replays shown, and each player's hit-by-the-ball state.

1. Start with shown = A, other = B.
2. If the point is **not** game-ending: swap them when there is a last hitter and A is not on the last hitter's
   team. (In a rally A is the last hitter, so this never swaps in practice.)
3. If the point **is** game-ending: swap them when A is not on the winner team. So a game-ending point always shows
   a player of the winning team.
4. Then, if replays shown is non-zero and a multiple of 3: swap again, **unless the player who would become shown
   (the current "other") is in the hit-by-the-ball state**; then keep the order from steps 1–3.

Consequences with one human (all follow from the rule, none is special-cased):
- Human hits the ball out / into the net / double-faults on an ordinary point: the human is the last hitter →
  the human is shown, with a loss reaction.
- CPU hits a winner past the human, or an ace: the CPU is the last hitter → the CPU is shown, with a win reaction.
- Human hits a winner or ace: the human is shown, winning.
- Any game-ending point: whichever of A/B is on the winning team is shown, winning (the human only if they won it).
- Every third replay of the match flips this choice (except onto a player hit by the ball).

Also at this moment, in doubles (more than two players on court): when the last hitter and the one before are on
the same team, the game sets a flag on the camera director. Its use was not followed; it can't arise in a singles
match.

## Rule 2: which reaction each player plays

At the reaction, for every player:
1. Hit by the ball → the hit-by-the-ball reaction.
2. Otherwise, under 2 players on court (practice) → the game-lost reaction in one practice condition, else the
   game-won reaction (already ported as `practice_short`).
3. Otherwise: on the winner team → win reaction (ordinary point) or game-won reaction (game-ending point); not on
   it → loss reaction or game-lost reaction.
4. In doubles, a win/loss reaction of an ordinary point may become a team reaction (already ported).

The shown player's reaction is simply theirs from this rule; nothing chooses a reaction to fit the camera.
No controller check anywhere in it.

## Rule 3: the shot picked for the shown player

As already ported in `cutaway::pick`: the down-the-line shots are skipped when the shown player's team lost (and
they aren't in a team reaction), and on a lost game-ending point the winner's shot is swapped for the loser's.
No controller check.

## Gap (what the port changes)

The port's step 4 holds the swap back when the other player is **diving**; the original holds it back when that
player is **hit by the ball** (the body-hit target this point). Diving players do get swapped onto. Fix: feed the
hit-by-the-ball condition (the point's body-hit target, as the reaction uses it) instead of the dive.

Everything else in the port already matches. The port session should:
1. Change the step-4 exception to "hit by the ball".
2. Prove the rule in a one-human match: record cut-aways from a save with P1 human (slot 3 or 4, driven with the
   virtual pad, e.g. the human returning some balls and missing others) with the same recorder as the P16a fixture
   (`tools/record_cutaway.py … --players`), and check the shown/other roles and the picked shots against the port
   like the all-CPU test does. Include at least one point the human loses by hitting out and one the CPU wins by a
   winner, and a game-ending point.
3. Side-by-side `tools/screenshot.sh` vs `--shot` at one such cut-away, to confirm by eye.

## Open questions

none

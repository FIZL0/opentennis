# M190 spec: a doubles computer player's formation spot is set up at the start of every point

Applies to every doubles computer player, in both builds (US and European). Singles is unchanged: it already has its
own set-up, the singles centre spot in `doubles_ai::run`.

## What the game does

Every doubles computer player has a formation state, which its rally routine reads and re-picks:
- a **lane**: 1 or 2, the side of its half it covers;
- a **forward** flag: it plays up at the net, or back;
- a **spot** (x, z): where its walk-back heads.

The hit message's own-team re-pick (play: `Rally::repick_as` via `heard_hit_doubles`) compares the spot after the
re-pick with the spot before. Only a spot that **changed** restarts the walk-back, and that restart rolls the
centre-rate chance, **one AI draw**. An unchanged spot draws nothing.

The game sets this state when the player enters its state for a new point: in the point's set-up, before the serve
and before any message of that point. This happens every point, in both builds. The values:

- **Lane and forward** depend on whether the player starts the point and on the court side being served from:
  - A player who starts the point (it serves or receives) is **not forward**. Its lane is **1 on the deuce court
    and 2 on the ad court**.
  - Its partner (the one who doesn't start) is **forward**. Its lane is **2 on the deuce court and 1 on the ad
    court**.
  - This is the same rule for the serving and the receiving team.
- **Spot x** = the player's end sign × 2.7425 × (+1 for lane 1, −1 for lane 2).
- **Spot z** = the player's end sign × −depth. The depth depends on the team's formation and the forward flag:
  - not forward: 11.0 (any formation);
  - forward, staggered formation: 3.2;
  - forward, any other formation: 7.9. The game's set-up checks "staggered" twice in a row, so the attacking
    formation's own numbers are never used here, and attacking and defensive both take 11.0 / 7.9.
  - The end sign is the same sign the rest of the formation code multiplies by. Play calls it `side` / `end`.

This is exactly what `hst_sim::position::Formation::start(team, starts, ad)` already computes. Use that, with
`starts` = the player is this point's server or receiver, `ad` = the point is served from the ad court, and the
team's formation, end and lean as `ai_wait` builds them. The port doesn't need new math, only the call at the right
time, writing lane, forward and spot into the player's rally state (`Rally::lane`, `front`, `spot[0]`, `spot[2]`).

Other things the same set-up does, which play already matches (the AI generator follows the recording through the
whole point up to the return's contact, so their draws are in place): it clears both shot records, makes the
partner's no-team redraw, sets the lean from the two players' styles, and makes the set-up timing draws. Don't
duplicate these.

## Order

- The set-up runs once per point per doubles computer player, before any hit message of that point can reach it.
  Play's first message of a point is the serve's hit message (and, on the match's first point, the toss announced
  as a hit). At that message the server's partner already re-picks. So the state must be in place before
  `at_message` runs for that player, not only before its first `run`.
- `set_up` (cleared on the serve message, `Shared::hear` → `heard`) is the natural per-point flag. Today singles is
  its only user. Either reuse it for doubles too, or keep a separate doubles flag cleared at the same place.
- The walk-back itself (`Rally::back`) is **not** set by the set-up. It keeps whatever it has until the first re-pick
  that moves the spot, or until the rally state's entry restarts it, as play does today.

## Why the EU point showed it and the US point didn't

Play left lane, forward and spot at their zero defaults: lane 0, not forward, spot (0, 0). So every first re-pick
in a point "moved" the spot and drew a roll.
- In the US point, the re-picks at the return happened to move the game's spot too, so the counts agreed.
- In the EU point the returner is player 1, the receiver on the deuce court: lane 1, not forward, spot
  (−2.7425, 11.0) (its end sign is −1). Its own-hit re-pick ("me" cue, partner's depth) keeps lane 1 and not
  forward, so it lands on the same spot. **The game draws nothing.**
- Play's player 1 started from zero. Its re-pick landed on lane 0, x = +2.7425 (lane 0 isn't 1, so it took the −1
  side) and counted as moved, so it drew one extra roll. That extra draw shifted every draw after it, so player 2's
  reaction came out 21, not 20, and its move started a tick late.

## Evidence (journal README)

- An AI-draw capture on `eu-bot-start.p2s` with a caller trace for every draw. At the return's hit message the game
  draws 18 (player 0: wait restart, 16 timing, reaction), **15 (player 1: timing only)**, 18 (player 2), 17
  (player 3: wait restart, the re-pick's walk-back roll, 15 timing). Play draws one more for player 1: its
  re-pick's walk-back roll.
- A snapshot of player 1's AI state with every draw: lane 1, not forward, spot (−2.7425, 11.0). It is set in the
  set-up, well before the serve, and is unchanged through the return's message.
- A throwaway experiment in play (reverted; not the port): `Formation::start` written into each doubles computer
  player's rally state the first time it runs or hears a message in a point. The results:
  - play's set-up values equal the game's for player 1 (lane 1, not forward, (−2.7425, 11.0));
  - the return's message draws **68** like the game;
  - recorded 250 draws 1 like the game. The first difference left is at 251 (play 3, game 1): player 2's walk-back
    after its first step, which is M185's stand scratch;
  - the `fifty_hz` tests (4) and the `doubles` tests (21) of hst-play all passed with it.
- The game's draws at 249–255 come from the two contact searches (the reach search, then the tiered search one tier
  per tick), not from a walk-back roll. So once M185 lands, compare those ticks against the search draws.

## Test

In `fifty_hz`'s `the_european_generators_play_as_recorded`, the AI generator should follow the recording through
recorded **250** (player 2's first step), not only through 229. The experiment showed only that the per-tick draw
counts match through 250. It didn't assert that the generator state equals the recording there, so the port
session checks that. The ticks past that wait
on M185. The US match's generator checks must stay as they are.

## Open questions

none

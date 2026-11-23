# B771 spec: the doubles server hears its own serve's hit message

Covers the one missing AI draw at game 2's first serve contact in slot 5's US bot match (server p1, receiver p0,
play tick 6814). See the B769 journal: the game draws 67 + 2 there and play 66 + 2, and a dummy draw anywhere after
p0's whole sequence puts play back on the recording.

## Background the port already has

- The hit message's per-player work in doubles (`doubles_ai::heard_hit` → `Rally::heard_hit_doubles`). On a real
  hit by its own team, a computer player runs the formation repick (`repick_as`). The cue is "me" (partner's
  position) when it hit the ball itself, else "other" (the hitter's spot). If the repick moved its formation spot,
  the walk back restarts and the row's centre rate is rolled once (`Formation::repick` → `Return::new`).
- After that hit work, each computer player makes its timing / picks redraw (and, after an opponent's hit, its
  reaction and guess), in player order (`ai_heard_stroke`).
- The point set-up places every computer player's formation spot, the server's included: back for the server and
  receiver, forward for the partners, lane by court side (`Formation::start`, `set_up_doubles`).

## 1. The hit message reaches a doubles computer player in every AI state

- The doubles AI's states are: choosing (only for the instant of a point's start), serve, receive and rally. Play
  already passes the message in receive and rally, so serve is the one state missing. Between ends no hit
  happens, so play's skip of messages there changes nothing.

- In the game, the doubles AI's hit-message work isn't gated on the AI's state. A computer player in the **serve**
  state hears it just as one in the receive or rally state does.
- Play passes it only to players in the receive or rally state, so the serving computer player misses it.
- The server is in the serve state at two hits only:
  - its own toss, which is a hit by nobody (B727): no team, so no formation repick and no extra work;
  - its own serve's contact, a real hit by its own team. **This is the missing case.**

## 2. What the serving player does on its own serve's hit message

Same as a rally-state player on a hit by its own team, minus the rally-only parts:

1. **Formation repick from its own hit.** The cue is "me", read from the partner's current position. The ball's
   depth and "our team hit" are as for any own-team hit. The spot it compares against is the one its point set-up
   placed; nothing moves it between the set-up and the serve.
2. **If the repick moved the spot** (x or depth differs from the stored one): the walk back restarts and the row's
   centre rate is rolled once (**one AI draw**). An unchanged spot draws nothing.
3. Then its usual timing / picks redraw (already in play). This is an own-team hit, so there are no pace,
   fast-ball, reaction or guess draws.

Not done for a serve-state player:
- the rally routine's return to waiting (that is rally state only, and only past the first shot);
- clearing the "held" flag (rally state only).

The receive state's and rally state's handling is unchanged.

## 3. Order within the tick

- Players hear the message in player order (0, 1, 2, 3).
- Within one player, the formation repick and its roll come **before** that player's timing redraw.
- At game 2's first serve: p0 (receiver: timing 9, picks 7, reaction 1). Then p1, the server: the repick moves its
  spot, so 1 roll, then timing / picks 15. Then p2 (16 + reaction 1 + guess chance 1: no fast-ball reaction took hold for p2, so its guess is rolled;
  for p0 it took hold, so p0 rolls no guess). Then p3 (repick roll 1 + 15). That
  is 17 + 16 + 18 + 16 = 67: the game's 67 of its 67 + 2 (play's 66), and it fits B769's finding:
  an extra draw after p0's sequence, before p1's.

## 4. Why the earlier serves show no such draw

- At the serves before game 2 (ticks 1271 and 3983, both by team 0), the server's repick gave back the spot its
  set-up had placed, so it drew nothing. The evidence is the experiment below: with the server hearing every serve,
  every phase through game 2's first serve still matches, so those serves added no draw. The rule is the same. Only the repick's result differs: game 2's first
  serve is team 1's first, with different players, lean and end. Which of these moves the spot doesn't matter to
  the port: the repick it already has works out the spot. The brief's candidates (team 1's first serve, a
  receiver with no serve history, the grade-4 serve, the far end) are not separate rules. The receiver's own
  hit-message draws in the original match play's: the serve-pace chance only with a serve history, then the
  fast-ball check, the reaction, and the guess skipped after a reaction.

## Evidence

- A throwaway change (reverted, not committed) let the serve-state computer player through to the hit work. It ran
  `cargo test -p hst-play --lib the_us_bot_match_plays_as_recorded -- --ignored --nocapture` with `HST_M183A1_OFF=1`
  and the B767 generator fixture present:
  - **29 of 112 phases match** (28 before), through the second serve's set-up;
  - the players match on every tick from 219 through 7386 (7387 is the first miss). The only misses before that are each point-over's last tick (the known
    ones plus 6941, the fault's point-over's last tick). The dummy draw of B769 reached 7285.
- With the same change, all 8 `fifty_hz` tests pass (`--include-ignored`). The EU bot match still matches 7 of 144
  phases, as before.

## Port notes (for the port session)

- Let the doubles hit message through for a computer player in the serve state too. Its own-team formation repick
  (with the roll on a moved spot) then runs before its redraw. Keep the rally-only parts gated on the rally state.
- Raise the US test's phase assertion to 29 and its player check to the range 219..7387 (7387 excluded). That check excludes each point-over's
  last tick: the existing list plus 6941.

## Open questions

none

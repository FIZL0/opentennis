# B771 (spec): the doubles server hears its own serve's hit message

## Task

In slot 5's US bot match, at game 2's first serve contact (server p1, receiver p0, play tick 6814), the game's AI
generator makes one more draw than play. B769 found that a dummy draw anywhere after receiver p0's draws puts play
back on the recording. This task finds the rule that makes that draw.

## Finding

The original's doubles AI does not gate its hit-message work on the AI state. Play passes that message only to
players in the receive or rally state, so the serving computer player never hears its own serve's contact.

When it hears that contact, the server runs its own-team formation repick, with the cue set to "me" (its partner's
position). If the repick moves its spot, the row's centre rate is rolled once. That roll is the missing draw. Its
timing and picks then redraw, as usual.

At the earlier serves (ticks 1271 and 3983, both by team 0) the repick returns the spot the set-up already placed,
so nothing is drawn there.

Candidates checked and ruled out, since the brief listed them: first serve by team 1, receiver with no serve
history, grade-4 serve, far end. None of them is the rule. The receiver's hit-message draws in the original match
play's: a serve-pace chance only when there is a serve history, a fast-ball check, a reaction, and a guess that is
skipped after a reaction.

## Evidence

A throwaway change, reverted and not committed, let the serve-state computer player through to the hit work. With
`HST_M183A1_OFF=1` set, the command was:

`cargo test -p hst-play --lib the_us_bot_match_plays_as_recorded -- --ignored --nocapture`

- 29 of 112 phases match (28 before). This runs through the second serve's set-up.
- Players match on every tick from 219 through 7386; 7387 is the first miss. The only misses before that are each point-over's last tick: the known ones plus
  6941.
- All 8 `fifty_hz` tests pass (`--include-ignored`). The EU bot match still matches 7 of 144 phases.

## Next

The port phase implements `SPEC.md` in this directory, from the spec and the recordings. No Rust was committed in
this session.

## Not verified / not 1:1

- The port is not done. This is the spec phase.
- Why team 1's server's spot moves (which of lean, lane or end) was not broken down. The experiment only shows that
  the repick's result differs for that serve.
- Singles is not covered. Singles has its own hit handling, which this change does not touch.

# B574 — CPU partners beside a human "won't go for an easy ball"

User report (2026-10-18): in doubles, a CPU on the human's team sometimes lets an easy ball go.

## How the original decides
A CPU beside a human gives way to the human's *shot record*: every frame while the opponents' ball comes, the
human's AI object (it has one only because its partner is a CPU) searches where the human could play it (smash, a
volley if it stood inside the service line when the shot came, else a tiered stroke) and writes that into the shared
records. The CPU's rally WAIT substate follows (`follow`, walks back to its spot) whenever the human's record is
valid and the CPU hasn't just been reset (or it plays in front on the ball's half); RUN gives way when the human's
record is earlier/nearer (`give_way`, `mate > 0`). The original never looks at whether the human actually moves or
presses: if the human *could* reach it, the CPU leaves it. A human who stands still while the ball passes within
reach loses the point with the CPU watching. That is the behaviour the user saw, and it is the original's.

## What was checked against the original (PCSX2 copy 3, slot 3: chars 6/4/3/11, court 4, P1 human idle)
- `research/b574_idle_partner.py <slot> <frames>`: P1 idle (taps ○ only to serve); counts the partner's follows on
  opponents' shots and points lost unreturned while following. `research/b574_trace.py <slot> <frames>`: per-frame
  trace of the partner's substate/follow/fresh/defer/chase, P1's record and search state, positions, the ball, and
  P1's facing/stamina/tick/run/velocity/move state. Port side: the same trace from a 1-human app run (the probe is
  kept outside git, `context/b574/probe.rs`).
- Function: `human_record` already replays bit for bit against tag-4 recordings (`human_record_matches_the_game`,
  idle and pad-driven human), and the partner's receive/NET/BASE calls beside it.
- Glue (`doubles_ai::human`), field by field against the running game: P1's facing, stamina, stamina tick, run
  frames and move state are the same as the app's for an idle human (40/0/0/standing; the original keeps a stale
  velocity, read only by the dive check, which needs a running player); the human AI object's level 3, mate 2,
  first entry 2, window 180 are what the app sets; reach stats equal `g.reaches` for all four players.
- Point start: in the original the partner's `follow` is clear when the next point's first opponent ball comes. In
  the app a server's partner keeps the last point's `follow` until the return is struck, then re-enters the rally
  that tick and clears it. The WAIT substate returns at once while only the serve has been struck, so it has no
  effect.
- One real gap: the smash stand offset sideways (`Body::smash_off[0]`) was one character's −0.067 for every player.
  The original measures it per character at load: the smash motion's racket point at the contact frame (x; its z
  is `smash_ahead`, already per character). Now `doubles_ai::smash_side` takes it from each player's own smash
  motion (a mod character's from its own), and it equals the original's per-player value bit for bit
  (`smash_side_per_character`, values read from slot 3's player objects: bd75cdd6 bd8949e8 bdc85d9e bd0301a8).
  It moves the CPU's (and the human record's) smash stand by up to 3.5 cm sideways.

## Numbers (idle human, P1 at the net or baseline as placed)
| | points | opponent shots with partner following | lost while following |
|---|---|---|---|
| original, slot 3 (49 points) | 49 | 20/89 | 11 of 35 losses |
| port, same characters/court (91 points) | 91 | 67/172 | 37 of 67 losses |
Per-shot, the port's opponents hit many more balls that pass the idle net human at volley height (first record a
volley on 16 of 33 traced shots vs 5 of 37), and each of those makes the partner follow, as in the original. The
traced cases looked at (port t8159–8243: ball passes 1 m from the idle human at volley height; t190: a high ball
over the net CPU that its own search can't take) follow by the same rules the original uses.

## Not verified / not 1:1
- Whether the port's opponents really aim at the net player more often than the original's: the two matches start
  from different states (the app can't load a PCSX2 save), so the shots differ and the follow rates (39 % vs 22 %)
  can't be compared shot for shot. The opponents' aim (`pair_aim`) is checked by the existing fixtures, but no
  same-state match comparison exists. A frame-exact doubles replay is P11k6's open part.
- The leftover `follow` at a point's start is cleared one tick later than in the original (at the return's tick,
  not before it). It doesn't change what the CPU does, since the WAIT substate returns at once until the return. Not
  ported, because that reset belongs to the point start's state order, which P11k6 covers.
- A moving (pad-driven) human's run/stamina fields were compared through the existing tag-4 fixtures only, not
  live against the app's `p.body.run`/`stamina_tick` in the same rally: no same-state run is possible (as above).

# B675 port: players' heads follow the ball

Ported from `SPEC.md` with the decompile locked.

## What landed

- `hst_sim::look` (`crates/hst-sim/src/look.rs`): the turn routine (`Look::turn`, one call: forward and rest angles,
  clip-end counter, re-seed, the two windows, the 7°/tick limit, the new head local) and `Look::reset`.
- `crates/hst-play/src/play/look.rs`: the gate each fixed tick after `bodyhit::pose`: the shared doubles contact
  hold (15 ticks from a contact tick, cleared at each serve set-up), the per-player latch (cleared and re-seeded on
  every motion set; mid-point, a blended set into a non-stroke or a non-whiff stroke turns it on; the server before
  the first shot in doubles, a dive or a smash fully resets instead), latch-off strokes (latch on 6 ticks after the
  player's own contact, else release, or no call for the server's first shot / dive / whiff / smash), the doubles
  release check. The new head local goes into `Posed`, so the next tick's body hit and the footstep effects read
  the turned head. State `Looks` is in the rollback snapshot.
- Drawing (B676a folded in): `character::HeadTurn` on each figure, the rotation R = rotY(nod)·rotX(turn) the look
  applied this tick; `character::animate` turns the `Bip01Head` joint by it in its own frame (the game's new local is
  R·clip local), for stock and mod rigs alike.

## Verification

Recording: `tools/pcsx2.sh python3 research/b675_turn_rec.py 5 2400 context/b675/port/run1.bin` (slot 5, the bot-only
doubles match; 2400 frames, none missed), copied to `fixtures/b675/run1.bin`. Per frame it holds every player's
motion player with its look state, the head node, its parent (neck) node, position, rally block and latch, plus the
ball, the hold and the match globals.

`crates/hst-sim/tests/look.rs` `turns_the_head_like_the_game`: on every settled tick it runs `Look::turn` from the
previous frame's state with the dumped neck world as the parent and the clip local from the dumped sampled rotation.
State and new head local come out bit for bit as the game's: 2231 tracking calls, 403 releasing calls (the doubles
check), 1618 idle ticks (latch off, not a stroke: no call, state untouched).

### Spec corrections found by the recording (behaviour, now in the port)

1. The head's position track has a single key, so the sampler never rewrites the local's translation row. The clip
   local's row 3 is whatever the look wrote last tick, with its round-trip drift. The test carries it over from the
   previous frame's head local. In the match, `Nodes` keeps it the same way (a one-key track isn't re-placed).
2. In the parent′ working copy, row 3's w is forced to 1.0 (as well as the mirror's negation).
3. Turn window bounds are `lo + (d + excess)` and `hi + (d + excess)` (the spec had `(lo + d) + excess`: 1 ulp off).
4. Clip done: clip length ≤ sampled time. The counter frames match that.
5. Release really is the raw (0, 0, 1). F and R are not remapped. Both confirmed.

### Skipped in the test (and why)

- Crossfade ticks (motion changed < 30 frames ago): the dumped sampled rotation isn't the mixed local the look saw.
- Ticks with the hold above 0 (one frame mismatched there, most likely the owner's re-run on a hit) and ticks
  where the players' tick didn't run (time frozen between points).
- The last tick before a motion change. At v7839, p0's serve clip sat at its end for a second tick: applied turn
  −0.0002, last angles unchanged. That doesn't fit a plain call (one would step to −7°). It's probably part of the
  motion change running that tick. Not explained.

## Not verified / not 1:1

- **Gate**: not checked tick by tick against the game. It follows the spec, and the recording's timeline fits it (the
  hold starts on the contact frame at 15, the latch comes on when it reaches 0). It approximates in four places:
  - the hold restarts when the match's shot count goes up, so a swing that misses on its contact frame doesn't
    start it;
  - "phase ≥ 2" is everything after the serve set-up tick (the game's set-up phase lasts ~5 frames);
  - a dive is `Player::dive`;
  - "since" counts from this player's last strike.
- **Motion change mid-point**: `Nodes::bind` puts the head's rows back to rest on a new motion. Whether the game
  keeps the last-written translation there isn't recorded.
- **Lefties**: no lefty in slot 5, so the mirrored parent′ path is unrecorded. The match passes the body hit's
  (mirrored-root) neck world as the parent.
- **Slow-motion factor**: always 1 (the exhibition rule's in-between frames don't run the look).
- **Drawing**: the turn applies per tick (not blended between ticks, like the arm IK). On a mod rig it turns the
  `Bip01Head` joint in its own frame, which assumes the standard skeleton's head axes.

# P115 — character 13's spot in the reaction voices' view test (FINAL)

Follow-up to P113 (journal 2026-10-21-p113-reaction-gate): the view test's second point is the player's position
for every character except 13 (Suzuki), which tests a point of its own.

## Getting a match with character 13

`tools/pick.py 13` failed on copy 2: in this copy ○ backs out of the menus and ✕ confirms, so its ○ presses
left the select. Picked by hand (slot 2, down, ✕, ✕ ×3 for the COMs, Start, ✕) and saved P1's first serve
(Suzuki, P1) to scratch slot 8. P1 then serves with ○ every 1.5 s (the in-match buttons aren't swapped).
`p3d1_view.py` is hard-wired to bot-only slot 5, so it wasn't used. Instead, small live probes
(`context/p115/watch*.py`, not committed) sampled all four players' spot field, the head bone and the reaction
state once per frame.

## What the point is

- It is **the head the post-point cut-away predicted** for that player: the Bip01Head position at the
  reaction's end (117 frames in, 378 on a game end), the same prediction the cut-away camera frames
  (`hst_sim::cutaway::subject_frames`, the cut-away recordings already read this field as "the predicted head").
- It is written for **exactly two players a point, the cut-away's shown and other player**, on the reactions'
  first/second frame, i.e. before the view test on the 5th. Every one of the 12 points sampled (3 min) wrote
  exactly two players. 117 frames later the live head bone sat on it to the last digit (difference 0.0) every
  time the reaction was still running then. When it didn't match, the next point had already started.
- A player who isn't a subject on a point keeps the value from the last point they were. Before a player's first
  cut-away it holds whatever was in memory (zeros for P1 in a fresh match, junk for the others).
- Values seen: about 0.8–1.3 m above the ground (game y is negative up), offset from the feet by the reaction's
  pose, e.g. (−0.06, −1.17, +1.11) from the position for motion 0x33.

So for character 13 the gate tests the predicted end-of-reaction head when they're in the cut-away, and a stale
head from an earlier point when they aren't.

## Port

- `cutaway::Director` keeps `heads` per player: the predicted head (`predicted(..).0`'s translation) of the shown
  and other player each time the director picks a shot. It's in the snapshot already (Director is), so rollback
  stays in step.
- `reaction_voices::spot`: character 13 → `Director::head(i)`, else the position. No head yet → the position.
- No play.rs change, no new draws, same system order.

## Tests

- `tools/check.sh -p hst-play reaction_voices` (matrix_is_local, character_13_tests_its_predicted_head) passed.
- `tools/check.sh -p hst-play cutaway` passed.

## Not verified / not 1:1

- **Before character 13's first cut-away** the game reads leftover memory (zeros or junk from an earlier match).
  The port uses the position there. That can't be matched (it depends on what ran before).
- **No lock-step app-vs-game check of the verdicts with character 13.** The identification is from live
  sampling of the game (12 points, two subjects each, head bone = field at reaction end). The app's prediction
  itself is verified by the cut-away tests (`cutaway2_s05`), but no recording runs the gate with character 13
  through the app.
- **Instant replays:** not checked whether the game also writes the field on a point that ends in an instant
  replay. The app writes it whenever the director picks a shot, which it doesn't do while a replay plays.
- `tools/pick.py` on copy 2 needs ✕ to confirm, not ○. Left alone (it's a shared tool and the other copies
  confirm with ○); noted here.

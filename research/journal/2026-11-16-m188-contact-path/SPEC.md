# SPEC — where the contact search gets its predicted ball (M188)

Clean-room spec: behaviour only. The port session changes play's contact searches to read the path below.

## The finding

A player's contact searches don't predict the ball themselves. They read the match's **path record**: the
predicted flight the AI already uses (P11n1 spec, `hst_sim::path`, ported and proved bit for bit). That record is
made once, at the launch, by a predictor copied from the live ball and grown 15 entries a frame. Play instead
re-predicts from the live ball every time it searches.

Before the ball's first court contact the two give the same positions. After it they don't, because the predictor
and the live ball bounce off different things:
- the live ball steps against the court's **mesh**. At a contact it sits at the mesh contact point less
  radius × normal.
- the predictor steps against the flat **court plane**. At a contact it sits at the sweep's centre with height 0.

The two placements differ in the last few bits: in the slot 5 serve, by −6.44e-6 in height and about 1 ulp in x and
z. The difference then rides along with each ball for the rest of its flight. A search made after the bounce (a
return, any ground stroke) therefore aims at the record's ball, which is a few ulps from the ball that actually
arrives. That is the 6e-6 M186 saw: the game's contact (−4.24201775, −0.78939843, 11.3753042) is the record's entry
for that frame, height negated, bit for bit. The flown ball (−4.2420173, −0.78940487, 11.375303) is the live ball.

## Which searches read it

These searches all read the record. Each one takes a **window**: the record's entries from `start` (the entry for
this frame's ball), up to the search's own count, cut short where the record ends.
- `n = min(count, end − start)`, or 0 when `start ≥ end` (record stopped, cleared, or run out).
- Entries are positions with height up (the record's own convention), plus each entry's court-contact count.
  The searches use the count where play uses `path_base + contacts`. The record keeps its counts across a net
  re-seed, so `path_base` isn't needed for these searches.

| search | count asked for |
|---|---|
| contact search on a press (smash / volley / ground stroke) | the player's reach horizon (its grade count), but at least 16 entries are taken for the distance pass. The stroke search itself runs over the first `min(window, horizon)` entries |
| the dive (last branch of the same press) | entries 5 up to `min(window, 16)` (the window being the one the press took above) |
| the auto-approach before a press | reach horizon + 20 |
| the late / whiff search | its own horizon (play's `whiff_horizon`) |

Each search's rules are unchanged (they are already ported in `hst_sim::swing`). Only the path changes: from "the
live ball now, then one plane step a frame" to "the record's window". When the window is shorter than play's path
today, the search sees fewer frames. That happens late in a flight, after the record's second predicted contact,
when it only tops up to 15 ahead.

### Edge: empty window
With no entries the stroke search, dive and whiff search find nothing. The auto-approach in the game then reads
stale memory for its direction, which is not reproducible. The port treats an empty window as "no approach". It
can only happen when the record is stopped, which a press on a live ball of the other team doesn't meet.

## Timing within a frame

The game's frame steps the ball, then updates the record (start + 1, grow), then runs the players. So when a
player searches, entry `start` is this frame's ball. Play already runs the record update first in `control`, so
the window is right as it stands at a player's turn. On the frame of a launch, HIT rebuilds the record (start 0, 15
entries) before anyone searches after it.

## What isn't changed

- The serve AI's search on the toss: it looks only before the toss can bounce, and before the first court contact
  the record and the live ball are identical (checked over every pre-contact frame of the two p11n1 recordings),
  so whichever source it reads, the values are the same. Leave it on the live prediction.
- The receiving human's smash-marker search has its own predictor (`Smash`). It is not this record.
- `predicted_contacts` / `path_base`: still used by whatever else reads them. The four searches above stop using them.

## Evidence

- `context/live/p11n1/doubles.bin` (+`.path`), `singles.bin`: each frame's record entry at `start` against the live
  ball (`scripts/record_vs_live.py`).
  - doubles: identical on all 752 frames before the first court contact; different on all 282 after it.
  - singles: identical on 634; different on 166.
  - The divergence starts on the bounce frame (7827 in doubles) and stays the same size (dx −4.77e-7, dy −6.44e-6,
    dz 9.54e-7) through the contact at 7864.
- The game's solved contact value appears in the doubles path recording's entries (74 times, the same entry seen
  across frames). It never appears as the live ball. The flown value appears only in the live ball (and twice in the
  path file, as entry 0 of the next launch).
- The search's count and window rules come from reading the press, approach and late-search code. The two
  constants (the at-least-16 count and the dive's 16-entry cap) were read from the user's GAME.BIN.

## How the port proves it

- `play::fifty_hz` `the_us_first_serve_set_up_plays_as_recorded`: the returner's slide 291–298 must now be bit for
  bit on x, not within 1e-5 (the contact solve's step should be the game's −0.12284347).
- The EU test (`serve_to_return`) must stay bit for bit through 248.
- The bot replay / snapshot tests may shift where a post-bounce search lands one frame differently. Check that
  any change goes toward the recordings, not away from them.

## Open questions

none

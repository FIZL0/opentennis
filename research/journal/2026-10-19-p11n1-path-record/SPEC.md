# SPEC — the AI's ball-path record (P11n1)

Clean-room spec: behaviour only. The port session builds it in hst-sim from this and the recordings.

## What it is

The match keeps one *path record*: a predicted flight of the ball in play, frame by frame, up to its second court
contact. The CPU players read it (via a per-frame copy) to pick where to run and whether to let an out ball go. It is
built by stepping a second ball, the *predictor*, with the same ball-step physics as the live ball. The predictor
steps as a predictor: it makes no sounds or events, and it does not touch the live ball's state.

## Contents

- **Entries**: a fixed buffer of 600. Each entry holds:
  - the predictor's position (x, y, z, w);
  - its velocity (x, y, z, w);
  - its court-contact count after that step (int).

  Y is stored negated (y up): both y values are multiplied by −1.0, so a 0 is stored as −0. The w words come out of
  the predictor as they are (1.0 for position, 0.0 for velocity in every recorded entry). An entry has a few more
  words; nothing writes or reads them (they keep whatever was there).
- **end**: the number of entries written.
- **start**: the window start. It counts the frames since the path began (see the update rules).
- **active**: whether the record grows.
- **mode**: the predictor's line call (0 none, 1 in, 2 out, as `Call`) as of the last entry written.
- **gap**: the predictor's line distance taken when the prediction first turns out (see below).
- The four shot records and the claim slot/frame are kept with it. Those are already ported (`World.records`,
  claim); only how messages clear them is listed here.

## Per-frame update (once per game frame, all phases)

1. If end > 0, add 1 to start. This happens even when the record is not active.
2. If active:
   - **end = 0 (fresh path)**: write entry 0 from the predictor as it stands, without stepping it. Set mode to the
     predictor's call. The gap is not considered for entry 0 (quirk). Then add 14 stepped entries (below), so the
     first build gives 15.
   - **otherwise**: if the last entry's contact count is < 2, add 15 stepped entries. If it is ≥ 2, add 15 only when
     end − start < 15 (the window is running out). Otherwise add none.
   - **A stepped entry**: step the predictor one frame and write it as entry `end`. Then: if the mode was 0 before
     this step and the predictor's call is now 2 (out), set gap to the predictor's line distance. Then set mode to
     the predictor's call, then add 1 to end.
   - The buffer is never overrun. When end passes 599 (after a batch), active becomes 0, and the record stays as it
     is until the next path.
3. Presentation markers are updated after the record (lob / landing marker effects: out of scope, the app has its
   own).
4. Doubles only: a countdown that resets to 15 when any player is in a swing state, and otherwise counts down
   to 0 (first it counts down by 1 if above 0, then it is set to 15 if a player swings). It is skipped when the update
   is run from a HIT (below). In singles nothing sets it, so it stays 0. Its only reader is presentation: each
   player's per-frame pose step turns the player's upper body/head toward the live ball only while the countdown is
   0 or less. So in doubles, nobody turns to look at the ball while anyone is swinging and for 15 frames after. That
   look turn is a bone adjustment on the drawn model. It does not feed the record, the AI, the ball or the shot
   code. For the port session: keep the countdown with the record (it is cheap and its clears are listed below), and
   expose it for a look-at-the-ball pose if the app adds one. No gameplay test depends on it.

Quirks to keep:
- **The gap is never reset.** It keeps its last value across paths and points; only an out-turning step overwrites
  it. Mode is reset to 0 by every path start and stop.
- **mode can go back.** It is the call of the last step, not a latch. In practice the call only changes at the
  predictor's first contact, so after that it stays put.
- The predictor keeps stepping while the record tops up after the second contact (third bounce and on). Mode and
  gap follow those steps by the same rule.

## The predictor's line call

The predictor's call and line distance come from the same ball step as the live ball's (judge.rs `call_landing`,
Flight call / line distance). The call is made at its first contact, only in the rally phase (3) or the special
modes the live ball also calls in. So on a serve's path (phase 2) mode stays 0. In a rally it becomes 1 or 2 at the
predicted first bounce.

## When the predictor is reseeded

- **Every shot launch** (a strike, a return or a serve hit that goes through the launch): the predictor becomes a
  copy of the live ball just after the launch. That includes its counters (all contacts, court contacts, special
  contacts), which the launch has zeroed, and its call reset. The launch then sends HIT, when the phase is not 0.
- **Every live-ball step that touches a special material** (the net / net-cord type), not only the first one. This
  is skipped in slow motion (the replay / slow-mo time scale). The predictor becomes a copy of the live ball after
  that step, except that its counters (all contacts and court contacts; port session: the special-contact count
  too, per the cord recording) keep the values they had in the predictor before the copy. The predictor's call is reset to 0. Then NEW_PATH is sent.
  - Effect: the live ball's counters are what the AI reads as "hits so far". The predictor's counters carry the old
    prediction's count, so the new path's contact counts (and its "stop at 2" rule) follow the old prediction's
    counts, not the live ball's.

## Messages (sent synchronously, any time in the frame)

| message | claim | record |
|---|---|---|
| NEW_PATH | cleared (−1) | active = 1, end = start = 0, mode = 0, the four records' n = −1. No build now: the next update does it |
| HIT | cleared | only when the phase is > 1 (serve 2, rally 3): marker flags; the shot-info landing quad = the live ball's position; active = 1, end = start = 0, mode = 0; **run the update now** (step 2 with end = 0, so 15 entries; the doubles countdown step is skipped); then the records' n = −1. Phase ≤ 1: nothing |
| STRIKE | cleared | active = 0, end = start = 0, mode = 0, records' n = −1, markers cleared |
| REACTION, and the one after it (`REACTION + 1`) | **kept** | as STRIKE |
| SERVE, CHANGE_ENDS, and the message 6 (point start) | cleared | as STRIKE, and the doubles countdown = 0 |

Order within a frame, from the recording: the regular update runs earlier in the frame than the hit. On the frame a
shot is hit, the record ends with end = 15, start = 0 (the HIT's own build). The next frame's update makes it
end = 30, start = 1. The CPUs that think after the hit see the 15-entry path.

## What the AI reads

- **The copy** (`Rally::copy_path`, ported): once per frame, the first CPU to think copies
  count = min(its window, end − start) entries starting at entry `start` (0 when start ≥ end). Only position, velocity
  and contact count are copied. The stamp is the frame counter. The window is 180 by default.
- **mode and gap**: the let-go / hold check is mode == out and gap ≥ the row's line margin (`World.path_mode`,
  `path_gap`).
- **The live ball's two counters** (all contacts, court contacts) as "hits so far" (`World.hits`). These differ from
  each other after a net touch.
- The shot records and claim (ported), with the clearing rules above.

## Differences from the app today (doubles_ai.rs / play.rs)

- The app builds a fresh 180-frame flight from the live ball every frame. The game grows one path from the hit,
  15 entries a frame from a predictor that keeps its own state. The game's window starts at "frames since the hit",
  and it stops growing at the second contact plus top-ups.
- The app's mode/gap come from that fresh flight. The game's come from the growing predictor, with the gap kept
  across paths.
- The app reseeds on the first net touch only, approximating the counters with `predicted_contacts` (`path_base`).
  The game reseeds on every net-type step, outside slow motion, keeping the predictor's counters.
- The app clears the records on every message, and the claim on every message except REACTION. The game keeps the
  claim on REACTION and REACTION + 1 and clears it on the others above. A HIT before phase 2 changes nothing in the
  record.

## Recordings for the port session

`tools/record_live.py <slot> <frames> <out.bin> --path` (P11n1 adds `--path`). Next to the live ball, the predictor and
the rally block of every frame, it writes `<out.bin>.path` with:
- the frame's vsync;
- the record's header: the buffer pointer, end, start, active, mode, gap, the four shot records;
- the claim slot/frame;
- the AI copy's stamp and count;
- the last up to 32 entries [first, end), read in the same frame. When that read ticks, first = −1 and no entries
  are written.

A port test can start from a recorded predictor and step it. For each frame it checks end/start/active/mode/gap and
that the tip entries match bit for bit (y negated, −0).

- `context/p11n1/run1/live.bin` (+ `.path`): slot 5 (bot doubles), 1500 frames, no frame missed. It shows serves
  (mode stays 0), rallies (mode 1 at the predicted bounce), growth to the second contact then top-ups of 15, the
  hit-frame end 15 / start 0, and the end of the point (inactive, end 0). Gap stays 0: no out ball.
- Still to record: a net touch (`--vel` a ball into the cord, as P15 did) for the reseed with kept counters and
  NEW_PATH; an out ball (poke the velocity so it lands long) for the gap and its carry-over into the next path; a
  long flight past 600 entries (e.g. a ball rolling with no second contact) for the cap. Also bots_singles, to show
  the same rules in singles (no countdown).
- The existing ai_rally fixtures already hold the header at each CPU call, which is a second check at AI-call time.

## Open questions

none

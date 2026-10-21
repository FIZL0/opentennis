# P131 spec: when a CPU server's serve state ends

Behaviour of the original, in our own words, from reading the game and checking it against the recording
`context/fixtures/ai_glue_s05.bin` (tags 5/6, slot 5, all four players CPU, two points). Terms as in the P11k6
spec (`research/journal/2026-10-21-p11k6-doubles-wiring/SPEC.md`): "state" = the AI's top state (choose 0, serve 1,
receive 2, rally 3); "the rally entry" = `Rally::enter_rally` (the Return roll, the put-none record, `Mind::rally`),
which the app already runs in the original's order (P130).

## The rule

The serve state's routine walks, waits, tosses and presses its serve (the app's `bot_serve`, already ported). After
the serve press it sits in its last step for the rest of the serve state. In that step, each AI update:

1. On the frame before the contact it aims (the app's serve aim, already ported). On no other frame does it touch
   the stick or the buttons: from the contact to the hand-over the CPU server gives no stick and no press.
2. It reports **done** when the server's **stroke flag** is clear (below). Done makes the AI update switch the state
   to rally and run the rally entry in that same update. The rally routine (NET/BASE, the app's
   `doubles_ai::step`) first runs on the next frame's update. On the hand-over frame the stick stays as the update
   started it (zero), so the serve routine's step and the hand-over give the player no stick that frame.

**Phase gate.** The serve routine does nothing at all, and is never done, unless the match is in the serve phase or
the rally phase (the app's `Phase::Serve`/`Phase::Rally`). If the point is over (or the ends are changing) by the
time the flag would let it go, the server **stays in its serve state for the rest of the point**: no hand-over, no
rally entry, no Return roll, no net repick for it in that point. The next point's new-point message resets the AI as
usual. Any point that is over by the hand-over frame (e.g. a serve fault called within recovery + 1 frames of the
contact) gives its server no rally entry.

Singles and doubles behave the same here: both modes' serve routines end on the same flag, under the same phase gate.
(What the singles rally entry draws, and in which order, is P132's.)

## The stroke flag (the player side)

The player has one "stroke in progress" flag that the AI routines read. For the serve:

- **Set** when the serve swing starts (the press that starts the swing motion, the app's `serve_turn` swing).
- **Cleared** by the player's own serve-state step on the first frame on which the frames-since-contact count has
  reached the serve's recovery (the app's `motion::serve_recovery`: 15 after a weak or underhand toss, a slice or an
  underhand serve, else 30). The count is 0 on the contact frame and goes up by one at the start of each later
  frame's serve step, before the test. So with a 30-frame recovery the flag clears on contact + 30.
- The clear does **not** wait for the stick or for the swing motion to play out. Those only decide when the
  follow-through hands off to standing or running (the app's `motion::serve_over`, already ported: past the
  recovery, and a stick or the motion's end). The flag can be clear while the server is still held in its
  follow-through; it stays clear from then on.
- It is also cleared at every point's reset.

## Order within a frame

Players are updated in order 0..3. In a CPU player's update the AI update runs **first**, then the player's own
state step (where the serve contact and the flag clear happen). So:

- the contact (and its hit message, 0x15) happens in frame H, after the server's AI update of frame H;
- the flag clears in frame H + recovery, after that frame's AI update;
- the AI update sees the clear flag, and hands over, in frame **H + recovery + 1**.

Recorded (both points of `ai_glue_s05.bin`, both full-strength serves, recovery 30): the server's hit message at v7784
and v8841, its update going 1 → 3 at v7815 and v8872: 31 frames after the hit message's frame. The second point
has a server update record on each of those 31 frames. The first point's recording has no records for v7795–v7799,
while v still counts one per frame, so count the frames by v, not by records. That
update makes exactly one draw (the Return roll; the repick keeps its pick, as P130 found).

After the hand-over the rally routine owns the stick. The server's held follow-through then lets go on the first
frame the rally routine gives a non-zero stick, or when the serve motion has played out, whichever comes first
(the app's `serve_follow`/`ai_serve_step` path; the frame order of P11k6 applies: the AI's stick of this frame is
what the player's serve step sees this frame).

## What differs in the app today (for the port)

- `ai_update` switches the Mind from Serve to Rally as soon as the game phase leaves Serve (the contact), and the
  rally entry waits for the first `doubles_ai::step`. That step is held back until the follow-through lets go
  (`ai_serve_stick`'s wait, 55–56 ticks after the hit in all-CPU runs). The original: the hand-over at contact +
  recovery + 1, in the serve or rally phase only, with the rally entry in that update and the rally routine from the
  next frame.
- In Post the app still enters the rally (a step runs there); the original never does if the point is over before
  the hand-over frame.

## Checks the port should make

- `glue_matches_the_game` (hst-sim `ai_rally.rs`): a serve-state update whose routine is in its last step leaves for
  the rally exactly on the frame (by v) that is recovery + 1 after the server's own hit message, with the recorded
  one draw. Every earlier serve-state update after the hit stays in serve. Both recorded points.
- An app test (all-CPU singles and doubles, as `play/doubles_ai/serve_over.rs`): each point the server's Mind goes
  Serve → Rally on the tick contact + recovery + 1, never in Post. On that tick the server makes the rally entry's
  draws; its first rally-routine step comes the next tick. A point that ends before that tick (force one, e.g. a
  fault) gets no rally entry for the server.

## Out of scope

- A serve swing that finds no contact (the CPU always has one: its press comes from its own contact search, P130)
  starts the count later, from the swing motion instead of a contact. Not needed for the CPU.

## Open questions

none

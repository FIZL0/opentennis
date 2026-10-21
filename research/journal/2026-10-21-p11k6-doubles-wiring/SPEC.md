# P11k6 spec: the doubles AI's glue (its per-frame update and its message handler)

Behaviour of the original, in our own words, from reading the game and checking it on the running game (recorder
`tools/record_ai_rally.py` tags 5 and 6, capture `context/fixtures/ai_glue_s05.bin`: slot 5, 1500 frames, two
points, all four players CPU). Terms use the app's names: "AI object" = the `Rally` state plus the `Mind`,
"AI generator" = `Rngs::ai`, the receive/NET/BASE routines = `Rally::receive` / `Rally::rally` (already ported and
proven call by call in `hst-sim/tests/ai_rally.rs`), "state" = the AI's top state (choose 0, serve 1, receive 2,
rally 3), "substate" = the rally routine's WAIT/RUN/READY/SWING, "set_sub(WAIT)" = `Rally::enter_rally`'s work
(the Return roll, the put-none record, `Mind::rally`, the reset of the substate's fields).

## Already right in the app (no work)

Checked against the game while writing this; each has its own proof:

- **Contact depth**: per character, from the arm table average (`motion.rs::arm_table_ram`, bit-exact per player).
- **Smash stand offset**: per character (B574 test in `doubles_ai.rs`).
- **Path mode and gap, path window**: from the path record (`path_mode_and_gap`, P11n1 `Path::window`).
- **Human shot records**: `doubles_ai::human` (P11k7).
- **Hold flag**: a new-path message while the shot count is 1 sets it; a serve message clears it (as `Shared::hear`).
- **Hit counters**: the routines read the ball's bounce and contact counters as they are at the call.
- **Shot memory of the timing draw**: the opponent history (last and the one before), the serve history (shifted
  only when the hitter's swing was a serve), the "our team hit" flag and the dive redraw (the app's `ai_seen`,
  `ai_serves`, `ai_hit`, `ai_dove`).
- **The caller's stick quad**: the player's update clears the stick quad and the button word every frame before it
  calls the AI update, and turns what the AI left there into stick bytes afterwards; the AI update clears the quad
  again when the AI is inactive. `Out::default()` each frame plus the inactive zeroing is exactly this. Recorded:
  every update entry has a zero quad and zero button.

## Who calls what, and when

Each frame, the players are updated in order 0..3. A CPU player's update calls its AI update once (after clearing
the stick quad and button word). A human's AI object (a human whose partner is a CPU) is never updated.

When a match message is sent, every player handles it in order 0..3; at the end of a player's own handling its AI
object (if any) gets the message. In replay mode the AI objects don't get messages. The messages that reach the AI
in a CPU doubles point (recorded): serve (0x0e), 0x0f, 0x10, 0x12, 0x13, hit (0x15), point over (0x17), 0x18,
reaction (0x19). 0x0e, 0x0f, 0x10, 0x12, 0x13 and 0x18 changed nothing in any AI object and drew nothing in the
recording.

The hit message is sent from inside the hitter's own update, after that player's AI update for the frame and
before the hitter's partner/opponents are updated. **At the time of the hit message the shot count and the last
hitter are still the previous shot's** (recorded: the serve's hit message sees shot count 0 and no last hitter;
the return's sees 1 and the server). Everything the handler needs about the new shot comes from the path object's
last-shot record (below), not from those two globals.

## The AI update (once per frame per CPU)

1. If the AI is inactive, clear the stick quad.
2. By state:
   - **choose (0)**: go to serve if this player serves, else to receive if this player receives, else to rally.
     Entering serve runs the serve-state entry (the AI's reset; the app's `Mind::reset` path, P11m2: 17 draws).
     Entering receive draws nothing. Entering rally does set_sub(WAIT) (one draw, the Return roll). Recorded at a
     point's first frame: server 17 draws, receiver 0, the two others 1 each.
   - **serve (1)**: the serve routine (the app's serve walk/toss). After the serve it goes to rally with set_sub(WAIT)
     (one draw); recorded: 1 → 3 with 1 draw, some 80 frames after the serve's hit message (the app's serve-to-rally
     hand-over already matches the replay tests; only check it in the acceptance test).
   - **receive (2)**: run the receive routine. If it returns true, go to rally **in the same frame**: set_sub(WAIT)
     (the Return roll, then the put-none record, then `Mind::rally`). Recorded: 2 → 3 with exactly 1 draw, and it
     happens after the opponents' third shot, not when the receiver's own return makes the count 2; so the switch is
     only the receive routine's return value. **Gap**: the app has its own rule in `ai_update` (shot count ≥ 2 with no
     swing and no contact → `m.rally`) and applies `m.rally` after the step, then enters the rally on the next step.
     Replace both with the routine's return value and the in-frame set_sub(WAIT), in that order.
   - **rally (3)**: the NET routine if this AI's play is net, else BASE (as the app does).
3. Then the stick quad / button word go back to the player (the app's locomote/press).

## The message handler

Per AI object, in this order:

1. **Base step** for serve, strike (0x14), hit (0x15) and new path (0x16): the path copy goes stale, the human
   search resets (the app's `Shared::hear`).
2. **Strike or new path**:
   - new path while the shot count is 1: set the hold flag.
   - in rally state with the substate not SWING: set_sub(WAIT) **now, at the message** (Return roll, put-none record,
     `Mind::rally`).
   - in receive state with the receive substate not SWING: back to the receive's wait (no draw: it marks itself fresh
     and drops the push-off).
   - new path while rallying: clear the held flag.
   **Gap**: the app sets `struck` and re-enters at the AI's next update. The draw happens in the same order relative
   to the AI generator only if nothing draws in between (other players' updates and messages do); the put-none record
   lands after record clears made in between. Re-enter at the message.
3. **Hit**:
   a. In rally state with shot count (the previous shot's, see above) > 1 and the substate not SWING: set_sub(WAIT) at
      the message (the app's `heard_hit` draws the Return roll here and `step` uses it next frame; the put-none record
      and the substate reset must also happen now, see 2's gap).
   b. Read the path object's last-shot record: hitter slot, the hitter's swing branch, shot type, the "close" byte,
      the special-serve byte, the hitter's position at the hit, the shot's target and the ball's velocity.
   c. **Own team's hit** (hitter on this AI's team): store it as the own-team memory, then **repick the formation**
      (`Formation::repick`): cue Me with the partner's current position when this AI's player is the hitter, else cue
      Other with the hitter's position at the hit; it uses the ball's depth (z) at that moment and "our team hit" = true. If the formation moved, roll a new Return (one draw), as `Rally::repick`. Then clear the
      "first hit" flag. **Gap**: the app has no own-team-hit repick.
   d. **Opponents' hit**: shift the opponent history (and the serve history when the swing branch is a serve) and
      set the voice flag (the app does both).
   e. The timing draw (the app's `ai_draw` and `ai_draw_guess`, with `Mind::own_hit` on the own team's hit).
      Recorded: 15 to 21 draws per AI per hit in total, a + c + e.
   f. Clear the held flag when rallying.
4. **Point over** (0x17): active = chance(50) (one draw). Recorded: one draw per AI.
5. **Reaction** (0x19): the net-rate nudge and repick (already ported as `Mind::point_result`). Recorded: only one of
   the four AIs drew (one draw) and changed its net rate.

Within one message, the four AIs are handled in player order, so their draws interleave in that order: AI 0's whole
handling (a, c, e) before AI 1's.

## Acceptance test (port session)

Replay `context/fixtures/ai_glue_s05.bin` (read through `hst_data::fixture`; skip without it):

- Before each recorded call, set the world as recorded: the players' positions and bodies, the ball, the score
  globals, the path object's entries (from the entry record's tail), the AI generator (from the header's block, then
  carried), and the AI object from the previous call's exit for the first call only; after that carry the port's own
  AI object.
- Tag 5 (update): run `doubles_ai::step` for that player; compare the AI object (the mapping `ai_rally.rs` already
  has, `rally()`/`bytes()`), the stick quad and the button word with the exit record, and the AI draw count.
- Tag 6 (message, id = the second argument): run `doubles_ai::heard_hit`/`heard` and the hit-message work for that
  player; compare the AI object and the draw count with the exit record.
- The NET/BASE stand scratch (the stack quad read uninitialised) comes from the record, as in `ai_rally.rs`.
- Done when every call of both tags is bit-exact. Keep the run under ~10 s (1500 frames, 11k records, 60 MB: read it
  once, compare each record as it streams).

## Open questions

none

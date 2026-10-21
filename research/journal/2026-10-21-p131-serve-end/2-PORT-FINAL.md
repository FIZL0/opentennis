# P131 — port phase

Ported from `SPEC.md` with the decompile locked.

## What changed

- `ai_update` (play.rs): a CPU server's serve state is over when its serve stroke is (`serve_stroke_over`): it has
  served (`shots > 0`) and its frames since contact, as its update sees them before this tick's step, have reached
  the serve's recovery (or its follow-through has already let go). This applies only in the serve or rally phase. So the
  hand-over comes at contact + recovery + 1, and a point that's over first keeps the server in its serve state.
- `bot`: from the contact to the hand-over the server runs `ai_served`: no stick and no press. Its follow-through
  plays on (`serve_follow`), and once it has let go the server stands. On the hand-over tick, `doubles_ai::hand_over`
  makes only the rally's entry (Return roll, put-none record, net repick): same step code, without the preamble or the
  routine. The routine (`doubles_ai::step`) runs from the next tick. Its stick ends the follow-through
  (`ai_serve_step`), as the spec says. The old wait (`ai_serve_stick`: hold until the return, then the reaction
  frames) is gone from the app. `hst_sim::motion::ai_serve_stick` stays, with its own test.
- Between ends, a server still in its serve state walks off like the others (the old fallback). The spec has
  nothing to say about the change of ends.

## Proof

- `hst-sim ai_rally.rs glue_matches_the_game` now also checks the rule against `ai_glue_s05.bin`. Every serve-state
  update after the server's own hit message stays in serve, except the one exactly recovery + 1 frames (by v) after
  it, which goes to 3. Both points pass (2 hand-overs). Recording detail: each point's serve state hears an earlier
  0x15 message (v7734, v8791) that stores no shot in the team's memory. The server's own hit is the one that stores
  its shot (v7784, v8841), so the check keys on that.
- `play/doubles_ai/serve_end.rs` (all-CPU singles and doubles):
  - `a_server_hands_over_at_recovery_plus_one`: the Mind goes Serve → Rally on exactly contact + recovery + 1, in
    the rally phase, never in Post. The server doesn't move up to and on that tick, and the rally entry is made on
    it.
  - `a_point_over_first_gives_no_rally_entry`: a fault forced 5 ticks after the contact keeps the server's Mind and
    step state in Serve until the next point.
- `serve_over.rs a_server_enters_its_rally_return_roll_first` now runs the server's hand-over tick on its own
  (`bot`), and checks the entry's draws and their order there.
- `every_point_enters_again` now skips points that start during a change of ends. The new timing gives more points
  in 5000 ticks, and one of them started between ends: 20 draws, the known change-of-ends gap (journal
  2026-10-07-p11-ai 26). The old code never reached one in its 5 points.
- `doubles_ai.rs singles_bots_go_back_to_the_baseline` (B15) now checks the set-up's walk-back spot on the tick it
  is made. It used to check it on every rally tick. A server whose net pick is Net now runs the NET routine's
  after-own-shot step off its serve, before the return, and that step moves the spot to ⅔ of the way back
  from the net reach (8.99). That is the routine's own behaviour, but the old timing never reached it there. The
  no-pinned-at-the-net check is unchanged.

Measured, the follow-through's let-go after contact: singles 32 (17 after a quick-recovery serve), on the routine's
first stick. Doubles 54–55: the routine gives no stick, so the motion plays out. Before the port it was 55–56 in both.

## Not verified / not 1:1

- **The let-go after the hand-over** isn't recorded. The glue recording has the AI's update and messages, not the
  player's follow-through. The app now follows the spec (the rally routine's stick, or the motion's end). The older
  lock-step model `motion::ai_serve_stick` (a wait for the return, then the reaction frames) no longer drives the app.
  Its recording test still passes on its own data, but the app's doubles servers now let go when the motion ends.
  That may differ from that recording by a frame or more, and it isn't checked.
- **Recovery 15** (slice, underhand or weak toss) comes from the rule only. Both recorded serves are strong.
- **The point-over gate** is checked with a forced fault in the app only. `ai_glue_s05.bin` has no point that ends
  before the hand-over.
- **Between ends**, a server still in its serve state uses the app's fallback walk. The spec doesn't cover it.
- **Singles**: no singles update recording exists. The singles hand-over follows the same rule (the spec says both
  modes match).

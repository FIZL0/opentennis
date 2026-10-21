# 27 — P130: serve-to-rally hand-over draws, serve aim frame

## What was wrong

- **Hand-over order, and more.** `ai_update` ran `Mind::rally` (the net pick and count) as soon as the serve was
  hit. `doubles_ai::step` was supposed to roll the Return when it entered the rally. But step only enters when its
  remembered state differs from the mind's, and it never saw Start or Serve, because it doesn't run in them. So from
  the second point on, the remembered state was still the last point's Rally. The server (and in doubles the two
  players who are neither server nor receiver) **never rolled their Return at all** on any point but the first.
  Measured: all-CPU singles and doubles runs showed the server's entry only on point 1. The recording
  (`ai_glue_s05.bin`, second point v8656) has server 17, receiver 0 and the other two 1 draw each. At v8872 the
  server's 1 → 3 update makes 1 draw, where the app made none.
- **Aim frame.** `bot_serve` drew the serve aim at the press whenever the AI's *own* contact pick (`ai_search`) was
  ≤ 1 frame off. But the swing `serve_turn` starts uses the timing table's search (`serve::search`). Its grades 0
  and 1 are always none (`swing::timing` clears them, and `serve::timing` keeps or copies them). So the real
  contact is always ≥ 2 frames after the press. A late press (large serve error) therefore drew the aim 2+ frames
  before the contact instead of on the frame before it.

## Fix

- `doubles_ai::track` (called in `bot` after `ai_update`) records Start/Serve as the step's last state. Every point
  now enters the rally fresh.
- `ai_update`'s serve state only switches the mind to Rally. The step's entry (`Rally::enter_rally`: the Return roll,
  the put-none record, then `Mind::rally`) makes the draws in the original's order. Coming from the serve, its net
  repick is kept (`x.mind = kept` stays for the other entries, whose repick the app's Mind already drew).
- `bot_serve` no longer aims at the press. The aim is always at `left == 2`, the frame before the contact.

## Proof

- Sim side: `ai_rally.rs glue_matches_the_game` already replays the recorded 1 → 3 update as `enter_rally` (Return,
  then `Mind::rally`), bit-exact.
- App side, `play/doubles_ai/serve_over.rs` (all-CPU singles and doubles, 5000 ticks each, ~4 s):
  - `a_server_enters_its_rally_return_roll_first`: on every hand-over, net_left is forced to 0 so the repick draws.
    The test calls the step and checks the net pick, the count and the Return against the original's order on a
    copy of the generator. It also checks that the old order would have given a different result on ≥ 2 of them.
  - `every_point_enters_again`: each point's first state entries draw 17 + 1 + 1 (the recorded 19).
  - `the_serve_aims_on_the_frame_before_its_contact`: with the serve error at 8 (the press as late as the AI's pick
    allows), every serve aims on the tick before its contact, ≥ 2 ticks after its press.
  - All three fail on the old code (checked by swapping the old play.rs/doubles_ai.rs back in).

## Not verified / not 1:1

- **No new ALL-style recording.** In the original the repick at the serve hand-over almost never draws: the count
  gives 2–4 hits, the serve's own hit takes one, and the hand-over comes 31 frames later, before the partner can
  hit. So an ALL-style server recording would show the same single draw as `ai_glue_s05`. The order with a drawing
  repick is the routine entry's own (recorded at the receive → rally and hit re-entries in the glue test). Only the
  app test forces it.
- **The hand-over's timing.** The original goes 1 → 3 31 frames after its serve's hit message (both recorded
  points). The app's server first steps its rally 55–56 ticks after the hit, when its serve follow-through lets go.
  That end condition is the serve routine's, which needs a spec session → **P131**.
- **Singles receive → rally** still draws the repick (after the receive routine) before the next step's Return roll.
  No singles update recording exists → **P132**.
- **Serve that ends the point** (fault, ace into a change of ends): the server's entry now waits for a step. A step
  runs in Post but not between ends, so on a change of ends the old code's draw there is gone. The original's
  change-of-ends draws are already a known gap (journal 26: ~69 extra draws).
- **Singles server's entry** is checked against the doubles recording's rule only (the singles WAIT entry is the same
  Return + `Mind::rally`). There is no singles update recording.

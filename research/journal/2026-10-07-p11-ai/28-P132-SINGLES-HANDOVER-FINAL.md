# 28 — P132: singles receive → rally hand-over

## Recording

`tools/record_ai_rally.py` tags 5 and 6 now also exist with `HST_SINGLES=1`: they hook the singles AI's own
per-frame update and its message handler. Both take the same arguments as the doubles ones, and their prologues
are safe to relocate.

- Run: `HST_SINGLES=1 HST_DRIVE=1 tools/pcsx2.sh python3 tools/record_ai_rally.py 8 3000 … 1 5,6`.
- Slot 8 = `context/recordings/singles_1p_serve.p2s`: P1 driven and serving, the COM receiving.
- Output → `context/fixtures/ai_glue_singles.bin`: 3032 frames, 688 draws, nothing dropped or unhooked.

**What it shows:** singles goes over the same way doubles does. In 6 hand-overs, the receiver's state goes 2 → 3
inside one update call, with exactly 1 draw (the Return roll; the repick in the entry didn't draw, because the
count was still > 0).

- **When it happens:** the hand-over is decided only by the receive routine's return value. It comes 31 frames
  after the COM's own return (v24722 → v24753), or right after P1's next shot.
- **The app's old rule went over at the wrong moment:** shot count ≥ 2 and no swing → `Mind::rally` in
  `ai_update`. That fired as soon as the return's contact ended.
- **The app's old order was also wrong:** the repick ran after the receive routine, and the Return was rolled at
  the next step.

## Fix

- `ai_update` no longer has its own receive → rally rule, in singles or doubles.
- `doubles_ai::step` goes over on `Rally::receive`'s true in the same frame, in singles as in doubles. The step
  runs `enter_rally` (singles `set_sub(WAIT)`: Return roll, then `Mind::rally`), and that repick stands.

## Proof

- **Sim:** `ai_rally.rs singles_receive_goes_over_in_its_frame` replays every receive-state update call of the new
  recording (1551 calls, 6 going over) bit-exact. It checks the AI bytes, the stick, the button, the records and
  the draw count. Each call runs: receive → `enter_rally` on true → the singles kind lock (button, then the stick
  on the frame before the contact) → the stick is cleared if the AI is inactive.
- **App:** `serve_over.rs a_singles_receiver_enters_its_rally_in_the_same_frame` plays all-CPU singles for 12000
  ticks. While the receiver is receiving, its net count is forced to 0, so the entry's repick draws.
  - On each hand-over, the step itself went over (sub WAIT).
  - In the tick's draws, the Return roll and the repick sit back to back in that order.
  - The old order doesn't fit there on at least 2 hand-overs.
  - The test fails on the old code: the step never went over itself.

## Not verified / not 1:1

- **Order of the update's last steps.** Within the update, the order of the singles kind lock, the inactive stick
  clear and the rally entry is taken from what the recording fits. No recorded frame has a hand-over and a press
  together, so the lock's order relative to the entry is unchecked (the app runs the entry first).
- **Recording size.** Only the COM's receive was recorded, in 6 hand-overs from P1's serves. A drawing repick at
  the hand-over (count 0) is never reached in the original. As in P130, only the app test forces it.
- **Message calls (tag 6).** Recorded, but not replayed. The singles handler's hit/strike/new-path work is covered
  by P11l6's own tests; replaying it needs a singles version of `glue_call`.

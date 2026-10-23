# B655: □ never serves or hits

Where □ reached the match: every source (keyboard Space, pad West, the touch pad's □, which comes in as a gamepad's
West, and the web touch pad, the same) latches `SlotPad::serve`, which goes on the wire as SQUARE. The shot latch
(`SlotPad::shot`) only ever takes ✕/○/△, so rallies were already safe. The leak was the human serve: `human()` took
`shot.or(serve_press → ✕)` as the toss/hit press, so □ tossed and hit like ✕. Now only `shot` serves; the □ latch
is still recorded and sent (the instant-replay request in `instant_replay.rs` reads SQUARE, parked with B72), but
nothing in the serve or the swing reads it.

Settings → Controls: the `replay` row (file name kept, so existing controls.txt files still load) shows as
"Menu: delete / swap". The touch pad and touch.js are unchanged (the task says so).

Tests: `serve_check::square_never_serves` (□ pressed 8 times over 170 ticks at 1P's serve: no toss, swing or serve;
✕ after it tosses). It fails with the old line. `controls::cross_serves_on_keyboard_and_pad` docs point at it.

## Not verified / not 1:1
- Not checked against the original over PINE: the user decided this behaviour (□ does nothing in a match), and
  controls are front-end, which doesn't have to match.

# M185 2: who writes the stand scratch (European capture)

The EU start state (fixtures/m169/eu-bot-start.p2s) loads in the current PCSX2, so the live capture blocked in
iteration 1 was done there, in scratch slot 8. US equivalents via the M169 function matches; US slot 5 fixture
values agree with the findings.

## Method

- `scripts/eu_stack_rec.py`: hooks the AI dispatcher (or player update) entry and logs the dead stack below sp.
  The stand routine's scratch is the dispatcher's sp − 0x20 (= player-update entry sp − 0x1f0; EU 0x1fff8c0).
- `scripts/eu_watch.py`: poor man's write watch: entry hooks on the player update, dispatcher, every player-update
  callee, the gallery manager (EU 3b6a20) and walker tick (EU 3b3f70); logs whenever the quad changed.
- `scripts/eu_watch_check.py` / `eu_watch_show.py`: statistics.
  Captures in `context/m185/` (eu_stack.bin, watch1–3.bin, pu_stack2.bin).

## Findings (research notes; the SPEC is clean)

- Dispatcher-entry quad == player-update-entry quad on 2384/2384 calls: nothing inside the player update before
  the AI touches it.
- Players 2–4 in order: the previous player's body-to-attachment hand-off (US 351c50 / EU 3669d0) builds a copy
  of the matrix at player+0x17f0 with row 0 scaled by the mirror factor (+0x12b4), using the scratch slot as the
  temp for row 0. 1785/1788. It is skipped when the global read by US 184600 (byte 0x2eefe8) is nonzero.
- First player: last walker's translation row spilled by the walker tick (US 39f160 at its sp+0x50); walker slot 3.
  US slot 5: (8.886850, 0, 12.959484, 1); EU (−7.631587, 0, 19.867296, 1). (0,0,0,1) on 4 frames + 5 more in
  eu_stack.bin (≈ every 65 frames: on those frames the walker ticks run but something under the gallery manager,
  14c630 called from 3b6b30, leaves the identity translation last). Walker tick returns early when raining
  (court +0x135 = 2/3) or disabled (+0x301).
- Toss tick: toss routine (US 37af20) launch-velocity local at its sp+0x60. EU vsync 38035.
- Hit tick: trail reset (US 33ea10, P9 journal) unpacks the trail colour RGBA word (table 0x3fc180 row, stride
  0x4c) with 327f70 into its local at entry−0x10: (39,128,39,70) = 0x27802746 kind 3, (62,14,127,70) =
  0x3e0e7f46 kind 0.
- Unknown: vsync 38329 slot 2 (0.0221, 0.1487, 0.0035, 0.1453).

## Not verified / not 1:1

- No port and no Rust: the next session ports from SPEC.md. Not shown that feeding these values makes the US probe
  run past point 1 (M183a1's experiment with (0,0,1,0) for the partner only got through recorded 322).
- What row 0 of the attachment matrix is in play's terms (which bone) is not identified; play has no posed skeleton.
- Rain / no-gallery courts, the skip flag, and the one unnamed tick: not observed.
- US slot 5 itself was not captured live (old save-state version); US values come from its RAM fixtures.

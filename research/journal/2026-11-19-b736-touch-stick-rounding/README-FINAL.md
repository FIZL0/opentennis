# B736 — round the touch stick before it goes on the wire

## Problem
An iPhone host on the on-screen pad sent a slightly different stick value nearly every tick (a thumb on glass never
holds still), so rollback's guess for a remote seat (its last pad held) was almost always wrong: up to 1190 re-run
ticks per 600 in `context/iphone-desync/run1/hst-log.txt`.

## What changed
- `hst_base::device::TouchStick`: rounds the on-screen stick's raw tilt (the touch gamepad's `left_stick`, before the
  pad driver's deadzone) to 32 directions × 8 lengths, holding each step until the thumb is 0.75 step past it
  (hysteresis), then applies the driver's deadzone to the rounded value. Rounding the post-deadzone bytes didn't work:
  the deadzone edge itself flickers an axis between centre and ~0.37 as the thumb wobbles (first attempt: 19 changes
  in the test trace; rounding the raw tilt: 9).
- `netplay::round_touch`: for each of this machine's seats whose slot holds an on-screen pad (`emote::is_touch`
  name: iPhone/Android touch pad and the browser's), the pad's `lx`/`ly` are replaced by the rounded ones before the
  pad is used locally or sent, in both lockstep (`gate`) and rollback (`rollback.rs`'s local pads; one call added).
  Controller and keyboard seats are untouched; offline play is untouched (only online gates call it).
- `seats.rs` needed no change (the seat→touch link comes from `controls::PadSlots` and the gamepad's name).

## Tests
- `device::tests::a_jittery_thumb_sends_few_changes`: 600 ticks near full tilt with ±2 px of a 60 px reach of noise,
  then a quarter turn: raw bytes change 546 times, rounded 9.
- `device::tests::rounding_keeps_centre_and_the_deadzone`.
- `netplay::tests::a_jittery_touch_stick_changes_its_pad_rarely`: a touch seat and a controller seat fed the same
  jittery tilt through `round_touch`: touch ≤ 10 changes, controller unchanged (> 400), buttons pass through. Each
  pad change is one wrong hold-last guess on the remote, so this is the rollback re-run count's driver; a full two-
  machine ggrs session with jittery sticks wasn't built (the mash generator only presses buttons).

## Not verified / not 1:1
- Not tried on a real phone online: no second device in an unattended run. Feel online: direction now moves in
  11.25° steps with up to ~8° lag before a step is taken, run strength in eighths; offline touch play is unchanged
  (rounding only runs in online matches), so there's no offline feel difference to check.
- Re-run counts in a real two-machine rollback session with a touch host not measured (see Tests); the unit tests
  show the pad changes drop from ~every tick to a handful per 10 s.
- Not a gameplay port: online-only input shaping, nothing in the original to match.

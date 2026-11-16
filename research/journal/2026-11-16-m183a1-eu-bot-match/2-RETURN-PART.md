# M183a1, part 2: after M184, both matches play as recorded up to the return (PART: blocked on M186)

M184 landed the serve from toss to contact. I ran both whole-match probes again. This time they get past the serve.
Both now part in the same place, the return. So the gap is general, not a 50 Hz one, and it is M186's
(spec first). The flip and the release note wait again.

## Fixed: the probe played the EU match on the wrong court's collision world
- At first the EU served ball left the recording at its bounce (recorded tick 225), by a few ulps in position.
  The velocity and spin after the bounce were bit for bit.
- I loaded the recorded ball state into `Flight` and stepped it with court 6's world from the EU disc: 20 375 of
  20 376 live steps in the recording were bit for bit, the bounce included. Stepping play's own state with a freshly
  loaded court 6 world gave the recorded position. Stepping it with play's world didn't.
- The cause: the probe's `Args` had `stage: None`, so play's collision world was court 10's, the default. `main.rs`
  sets the stage to the court when only `--court` is given (B640). The US match is on court 10, so it never showed
  there. Both probes now set `stage` to their court. Test-only change: play itself was right.

## What matches now
| | US (60 Hz, slot 5) | EU (50 Hz, M177) |
|---|---|---|
| serve set-up and serve | as in M184: to the contact (219) | as in M184: to the contact (141) |
| served ball, through its bounce | bit for bit to the return's contact (298) | bit for bit to the return's contact (230) |
| players | all on the spot through 290 | all on the spot through 228 |
| where they part | the returner (player 1) slides too far from 291 | the returner (player 1) slides 0.0948 a tick too far from 229 |
| the return | launch ~4e-4 off from 299 | launch ~8e-4 off from 231 |

The US probe's generators still match the recording through 298 and part before 320 (M185, the net partner's
scratch target).

## Proof (`play::fifty_hz`, in the suite)
- New `serve_to_return`: the served ball bit for bit from the contact through the bounce to the return's contact,
  and every player bit for bit on the ground from the contact to the tick before the returner's slide. Both
  first-serve tests call it: EU (141, 228, 230), US (219, 290, 298).
- Checked it bites: with `stage: None` the EU test fails at tick 225, the served ball.
- New probe helper `first_point_part`: where a player and the ball first leave the recording in point 1 after the
  serve's contact, with the offsets. Both ignored whole-match probes print it.
- The phase compare is still 1 phase in both: point 1 ends at a different tick (EU 303 recorded vs 279 played,
  because the return differs).

## Next
- M186 (spec first): the returner's slide into the return and the return launch. I added a note there that the EU
  match parts the same way, so M186 should check both probes.
- Then M183a1 resumes: run both probes, chase the next gap (the US one hits M185 at 318), and flip
  `match_speed::FIFTY_HZ_SIM` with the release note once the EU match plays as recorded.

## Not verified / not 1:1
- **Whole-match proof not done.** Both matches part at the return's slide (M186). The US match also has M185 later
  in point 1. `FIFTY_HZ_SIM` is not flipped and there is no release note.
- **The first 8–9 ticks of the held ball** differ in both builds (the hand pose as the walk starts), as before.
- **No generator check for the EU match.** M177 recorded no `rng_*` file. The seed is still proven only through the
  weather schedule, the server's walk spot, and now a bit-for-bit serve to the return.
- **The US baseline's `rand()` start is computed**, not recorded (slot 5's intro was skipped), as before. It is checked
  against the recorded generators every tick to 298.

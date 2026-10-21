# P128 — the contact normal on a repeated ghost touch: spec

Clean-room spec (PLAN.md *Clean-room ports*). It describes behaviour only. The evidence is the game's live-ball code
and the live-ball recordings in `context/fixtures/` (`research/ghost_touches.py` lists every ghost touch they hold).

## Result: the case can't happen, and the port already matches

The P123 spec says the latest contact normal "is written for every contact the sweep finds. That includes a repeated
ghost touch whose response is skipped." The second half is true of the step's code, but that code never runs for any
ball. **The world sweep never reports a second ghost touch in one step.** So no ball can come to rest right after a
double ghost touch, and there is nothing to record. The port's world sweep already does what the game does:

- **The first sub-step sweep with no ghost touch yet this step** is unfiltered. A ghost triangle can be the nearest
  contact it reports.
- **Every sub-step sweep after a ghost touch in the same step** is given an ignore list that holds just the ghost
  material (32). The ghost-touch count is reset at the start of every step. It is not reset by a launch.
- **The ignore list works for every triangle the world sweep tests.** That covers the court and the props, the
  plain meshes and the ones that read a triangle's material from a material map. A triangle that the sphere meets is
  tested for its material only after the hit has been found:
  - The material is the triangle's own. A material of 0 falls back to the model's default material.
  - If that material is in the ignore list, or it is 0, the candidate is thrown away.
  - The hit record is put back exactly as it was before that triangle was tested: distance, point, normal, material
    and the rest. So an ignored ghost face can't shorten the search, and it can't leave its normal behind.
  - The sweep goes on and reports the nearest non-ghost contact, if there is one. That is the "real contact behind
    it". Its normal and material are the ones the step records.
- **The step's skip branch never runs.** The step counts ghost touches. When a contact is a ghost and the count goes
  past 1, it writes that contact's normal and skips the response. Only an unfiltered sweep could reach that. The step
  has one unfiltered variant, for a mode flag. No call sets that flag: every one of the game's calls into the ball
  step passes it clear, and the step isn't reached through a function pointer. That covers the live ball, the path
  predictor and the body-hit plane step. The plane step can't meet a ghost anyway: it sweeps a plane of material 0.

The port's rule is *after touching the ghost once in a step, the rest of that step's sweeps ignore it* (`mesh.rs`,
`GHOST`; `ball.rs`, the world query). That is the game's rule, in the same order. **No behaviour changes.**

## What the recordings show

28 live-ball recordings (every frames_live file in `context/fixtures/`), 85,199 frames (`research/ghost_touches.py`):

- 85,177 frames with no ghost touch and 22 frames with exactly one. No frame has two or more.
- All 22 end with the ghost as the step's latest contact: the stored material is 32 and the normal is the ghost
  face's. No recorded frame has a ghost touch followed by a real contact in the same step.
- The ghost touches are in `p7e_c07` (11), `p7b_c01` (4), `p7b_c08` (3), `p7b_c03` (2) and `p7b_c13` (2). A ghost
  touch counts as a contact for the classification: 21 frames leave it at *bounce* and one at *roll*
  (`p7e_c07` frame 1548). That agrees with P123.
- Twice the ball touches the ghost in two steps in a row (`p7b_c01` frames 351–352, `p7e_c07` frames 1547–1548), once
  in each. This shows the count is per step.

## For the port phase

There is no behaviour to port. Pin it with a test instead:

- In `line_calls.rs`, or a test beside `live_ball_comes_to_rest_as_the_game`, step every recorded live-ball frame
  the way that test does. Compare the port's latest contact normal (`Flight::normal`) with the recorded one, bit for
  bit, on every exact step. Do it on the 22 ghost frames for sure. The recordings that hold them are `p7e_c07`,
  `p7b_c01`, `p7b_c08`, `p7b_c03` and `p7b_c13`. The test's own list already has `p7b_c03`, `p7e_c07` and `p7b_c13`.
  Add the other two. Both are frames_live recordings.
- Also assert that in no recorded frame does the game's ghost count go past 1.
- Fix the P123 wording: the normal is written for every contact the sweep reports, and the sweep never reports a
  second ghost in a step. Fix it in this task's journal, not in P123's. Leave the old file as it is.

## Not verified / not 1:1

- No recording has a ghost touch with a real contact behind it in the same step. The order (ghost ignored, then
  the real contact) is read from the code. It is not observed. A PINE recording that aims a fast ball through a
  ghost face (a bush) into a wall or the ground in one frame would show it. I didn't make one: the code leaves no
  other path.
- The world sweep's triangle test runs on VU0 microcode, so the oracle can't run it. The ignore-list behaviour is
  read from the code and backed by the recordings above. It is not oracle-checked.

## Open questions

none

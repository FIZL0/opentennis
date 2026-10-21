# M7 — alternate match camera: spec

Clean-room spec (PLAN.md *Clean-room ports*). It describes behaviour only. Observations were made live over PINE in
PCSX2 copy 3 from save slot 3 (doubles, P1 human, the rest CPUs). Captures are in `context/m7/` (`shots.txt`: the
three scripted-shot records and scripts; `point_s03.txt`: the director's state through a point; `mode0.png` /
`alt.png`: the same moment in both cameras).

## What the player sees

There are two match cameras.

- **The normal one** is the match camera we already port (`hst_sim::camera`, mode 0). It is turned so the human's
  end is nearest.
- **The alternate one** is a low "behind the player" view. The eye sits a few metres behind the human and a little
  above head height, and looks past them down the court. The field of view is wider: half-angle 35° against the
  normal 20°.
  - In the slot-3 serve position the human stood at (3.0, 0, −12.25) (game space, Y down).
  - The alternate eye settled at (3.31, −5.58, −23.99) with a half-angle of 35°.
  - The normal camera's eye was at (0, −11.89, −39.24) with a half-angle of 20°.

Select switches between the two cameras. The change is not a cut: the view glides there over 15 frames. Pressing
Select again glides back over another 15 frames.

## The toggle

The game checks once per frame for a Select press (the edge, not the held state). The press counts only when every
condition below holds:

- the match is live: in the serve, the rally or the post-point stretch. Menus, pause and the score/result screens
  don't count;
- no instant replay or replay playback is running;
- no camera glide is running (neither a Select glide nor the change-of-ends glide);
- the match options allow the alternate camera. Retail always allows it. Our rule is in *Port rules* below;
- at least one player is human;
- **only player 1's pad counts**: the first pad's Select.

When a press counts, the camera switches to the other kind:

- **normal → alternate**;
- **alternate → normal**. The normal camera is freshly reset (the same reset a new point does), so it starts from
  its usual resting framing.
- With no human to follow, the camera is always the normal one.

**The choice persists.** It is kept across points, games and the change of ends. It is also remembered for the next
match: a new match starts in whichever camera the last one ended on. When the director resets at the start of each
point, it re-applies the current camera with no glide.

Second spec pass (re-read of the original's toggle, 2026-10-21) confirmed the rules above and added these details:

- The game keeps a camera *kind*: normal, alternate, plus an "unturned normal" kind used only when the
  fixed-end camera option is on (retail never turns it on in a match; our build has no such option). The toggle maps
  normal or unturned normal → alternate, and alternate → normal. A press while there is no human to follow is
  ignored in effect: the kind setter turns any kind into the normal camera when nobody is followed.
- A third "follow the human" kind uses shot 7 (the related variant in *Framing*). Select never enters it or leaves it,
  and nothing in a normal match selects it. Don't port it.
- The remembered choice is stored as one flag: on for the alternate camera, off for either normal kind.

Quirk: at power-on the original seeds that remembered choice from one random bit, so a fresh boot sometimes opens a
match in the alternate camera. The port doesn't copy this (see *Port rules*).

## Framing and follow (the alternate camera)

The alternate camera is one of the scripted shots in the game's camera-shot table, the same table and runner as the
post-point cut-aways (`exe::Game::camera_shots()`, `hst_sim::cutaway::Shot`).

- **Shot 8** is used in singles; **shot 9** in doubles.
- Each shot plays for as long as the alternate camera stays selected. Its script never ends it.

Both shots aim at the followed human:

- **Subject frame:** the human's own frame (their ground spot, facing their end). This is player n's ground-spot
  frame, the same frame the cut-aways use.
- **Look-at frame:** also the human's frame, reached through a second slot that the director fills in. In the live
  game both slots read as identity until the first Select and point at the human afterwards. Feed the human's ground
  spot into both.
- **Target:** the live ball frame. This is what the script's pitch-follow uses.

The script values below replace the shot's defaults. The record fields that 8 and 9 share are listed after them.

| | shot 8 (singles) | shot 9 (doubles) |
|---|---|---|
| pitch (tilt) | −15° | −16° |
| eye-height/framing scale (script's second value) | 0.85 | 0.75 |
| distance | 11 m | 13 m |
| horizontal half-angle | 35° | 35° |

Record fields shared by shots 8 and 9 (shot 7 is a related unused variant: −19°, 0.8, 12 m, 40°; yaw framing
0.8, 2, 2):

- the pitch follows the target (the ball), with pitch framing weights 0.9, 1, 1;
- the yaw follows with framing weights 0.8, 1.3, 1, and the yaw is clamped;
- the shot's "keep on subject" flag is on.

The cut-away runner already reads all of these from the record, so the port gives it the human's frame and the ball
frame and lets it run.

Behind the human means behind their end. When the human changes ends their frame turns with them, so the camera
follows to the other side of the court.

## The 15-frame glide

The glide runs on a Select press only. A point-start reset never glides. There is one case where Select switches
with no glide: the camera is pinned to a fixed end (the normal camera's unturned form) and the human is on the +z
end. That never happens in our build, because the normal camera always turns to the human. Note it, don't port it.

How the glide works:

- **At the press:**
  - store the view on screen right now (the eye, its three axes and the half-angle) as the start;
  - set a counter to 15.
- **Each frame:**
  - work out the new camera's view as usual (the alternate shot, or the reset normal camera);
  - count the counter down;
  - t = 1 − counter/15, so t runs from 1/15 up to 1;
  - weight w = (t^(1−t))². Same shape as the change-of-ends swing weight, but the inner exponent is 1 here, not 2;
  - the view shown is the start view blended toward the new view by w;
  - the blend is the change-of-ends swing's orbit blend about the human's frame, not a straight line between eyes:
    eye position, heading and pitch are each blended as orbit parts around the human's ground spot, then rebuilt;
  - the half-angle is blended linearly by the same w;
  - when the counter reaches 0 the glide ends and the new camera runs on its own.
- While the glide runs, Select is ignored.
- Live, the glide took 14–15 game ticks each way.

## Every-frame extra in the alternate camera: fading the human when they block the ball

The camera sits right behind the human, so their body can hide an incoming ball. To keep the ball visible, the game
fades the human toward half transparency.

1. Each frame, before the check, the human's body, any second model and their attachments (racket and the like) are
   reset to opaque. The fade is then applied to all of them together.
2. Project the human's ground spot and the ball's centre to the screen, in pixels of the 640×448 output (the
   vertical coordinate is doubled from field lines).
3. The ball counts as **blocked** when all of these hold:
   - it is the rally or the post-point stretch, and not within the first 30 ticks of the rally;
   - the ball is farther from the camera than the player: (ball − player) · (player − eye) > 0;
   - their screen x differ by less than 40 px;
   - the ball is above the player's spot on screen by more than 0 and less than 200 px;
   - the human was not the last to hit the ball (it's coming to them, or on its way back from a partner/opponent).
4. A counter from 0 to 5 rises by 1 each blocked frame and falls by 1 each other frame. In the post-point stretch
   it is forced to fall.
5. The human's alpha = 1 − 0.5 × counter/5: from opaque down to half transparent over 5 frames.

The normal camera never fades anyone.

## Stick directions and gameplay

- **The alternate camera changes nothing in gameplay.** Stick input is never rotated to the view. The pad's only
  link to the camera is the same one the normal camera has: the stick is flipped (turned by 180°) while the eye on
  screen sits at z ≥ 0, and is straight otherwise. The alternate camera is always behind the human, so it flips the
  stick exactly as the turned normal camera does. The direction you push means the same thing in both cameras.
- Nothing else reads the camera kind: shots, AI, scoring and match timing are the same in both.

## Through a point (observed)

From slot 3, the alternate camera was switched on, then the human served and the point ran out. Every sample showed
the alternate camera:

- before the serve;
- through the rally;
- through the post-point stretch;
- after the next point's reset.

No cut-away mode appeared in the post-point stretch, so with the alternate camera on, the post-point cut-aways
**don't play**: the alternate camera stays. On the next point's reset the camera re-applied the alternate camera
straight away. One sample showed the normal camera's eye for a single frame at the reset, and the alternate view
returned on the next.

## Port rules (from the task and the project rules, not from the original)

- **Who can toggle it:** only when exactly one local human plays. That rules out local multiplayer and online
  matches; there the binding does nothing in a match and the normal camera stays. Retail lets player 1's Select
  toggle it whenever any human plays. The task restricts it.
- **The key:** our `camera` bind (key C / pad Select, `play/controls.rs`). Today it only swaps the hand at
  character select; in a match it toggles this camera, read from the one local human's own input. Update the bind's
  description and the controls text.
- **Shown view only:** the alternate camera is presentation.
  - It must not change `Game::cam`, which is in the state hash and is what reaction voices and footsteps test
    against, drawing the sound RNG.
  - It is a shown-view copy, layered like `play/own_camera.rs`: the sim keeps stepping the normal camera exactly as
    now, and the drawn view (and the stick flip, via `own_camera::flipped`'s shown eye) comes from the alternate
    one.
  - With it never selected, a match plays bit for bit as before, and the replay tests stay exact.
- **Remembered choice:** keep the last choice in the player's settings (default off) instead of the power-on random
  bit.
- **Rendering:** the fade needs the human's meshes to draw with alpha. Match it by eye.

## Not verified / not 1:1

- **Change of ends in the alternate camera:** not recorded. Expected behaviour: the human's frame turns, so the
  shot follows them to the far side. Not observed: whether the change-of-ends glide plays first (the global "no
  glide running" condition suggests it can), or whether the camera snaps at the reset.
- **Post-point with a long rally ending:** only a short point (a quick serve/fault) was observed. That cut-aways
  never play in the alternate camera is inferred from the director mode staying on the alternate camera through it.
- **The rally follow:** the human didn't move in the observed sample. The yaw/pitch follow comes from the shot
  record via the existing runner and was not watched live.
- **The fade:** never triggered in the sample (counter stayed 0). The rule above is not confirmed on screen.
- **The look-at slot:** that it holds the human's frame is inferred from both slots going from identity to set at
  the toggle. It was not read back as a matrix.
- **The Select-glide weight curve and timing:** the 15-frame length was seen live; the weight curve was not
  checked frame by frame.
- **Multiplayer:** whether retail follows player 1 or someone else with 2+ humans was not resolved. The port's
  one-human rule makes it moot.

- **How the port settles these:** by watching PINE recordings (change of ends, a long rally ending, the fade). That
  needs no decompile.

## Open questions

none

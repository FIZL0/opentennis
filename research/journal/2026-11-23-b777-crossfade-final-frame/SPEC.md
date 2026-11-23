# B777 spec: the crossfade's last frame and the switch-tick bind

Scope: the motion player's normal crossfade, and the doubles stand test's same-tick read of player 1's Racket chain
(B773). Two behaviours of the game explain B773's "crossfaded motion" misses. Both are read from the original's
motion step and motion setter, and both are confirmed by the fits below.

## Terms

- **Body tracks**: the skeleton's main body tracks, the ones a crossfade mixes. That's the first 23 bones in clip
  track order; extras such as hair and props aren't mixed. Same set play's fade mixes today.
- **A track's own value this frame**: what the new clip gives a track at its clock.
  - A track with more than one key: its sample.
  - A track with exactly one key: that key.
  - A bone the new clip has no track for: its rest transform.
- **Bind**: what happens when a different motion id starts (section 2).

## 1. A crossfade's last frame still mixes, at weight 0

A normal crossfade starts on the tick the new motion starts, which is the new clip's time 0. It runs for 8 motion
steps. On each step the game:

1. advances and samples the new clip,
2. samples the old clip at the old clip's own clock,
3. for every body track, sets the bone to the mix of (the new clip's own value this frame → the old clip's sample)
   at the step's weight,
4. counts down. Only once the count has run out does it drop the old clip.

| step (new clip time) | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 on |
|---|---|---|---|---|---|---|---|---|---|
| old-clip weight | 7/8 | 6/8 | 5/8 | 4/8 | 3/8 | 2/8 | 1/8 | **0** | no mix |

Step 7 is where play differs. Play drops the old clip as soon as the weight reaches 0 and skips step 7's mix. The
game does mix at weight 0, so every body track is set to the new clip's own value this frame.

For bones the new clip samples every frame this makes no difference. For single-key and untracked bones it decides
the result, because nothing else rewrites them after the bind. Without step 7 they keep step 6's 1/8 blend for the
rest of the motion, and that blend depends on the previous motion. In the US probe the right clavicle stays 0.079
off after stand→turn_fore and 0.055 off after run→turn_fore, which moves the racket z by up to ~0.09.

After step 7 the pose depends only on the new clip and its clock. The clavicle, neck and pelvis rotation are
single-key in turn_fore, the turn clips and run_forward, and the racket bone has no track in them, so the whole
Racket chain is affected.

Exactness: the result of step 7 is the new clip's own value. In the probe, play's existing slerp/lerp at weight 0
gave exactly that (bit-exact racket z on the matched ticks). Copying the value is equivalent, so the port may do
either.

Applies to:
- the stored pose's motion step each tick,
- the stand test's same-tick read of player 1, which repeats the motion step and so must also do step 7,
- the drawn pose, which shares the stored pose's motion step. Rendering only has to look right, but it follows from
  the motion step anyway.

A new motion that starts while a fade is still running is unchanged by this spec. Keep play's current handling.

## 2. The tick a new motion starts binds it before its first step

When the motion id changes, the game binds the new clip before that tick's step 0. Binding means:
- every bone's transform goes back to the skeleton's rest pose,
- every single-key track's bone is set to its key,
- untracked bones stay at rest.

It does this only when the id actually changes; a repeated command for the same motion doesn't bind. The old clip
and its clock, which the crossfade samples, are kept as they were before the bind.

Play's stored pose already binds on a change. The stand test's same-tick read doesn't: when player 1's motion command
starts a new motion this tick, the read samples the new clip over the old motion's leftover bones. On a switch tick
the read must bind its working copy of player 1's pose first. The read is a look-ahead, so it must not change player
1's stored state. This was the switch-tick miss (US 888: game 0.4028, play 0.4199).

## Evidence (B773's fit scripts, same-tick read, B773's tick numbers)

| probe | misses before | with 1 | with 1 + 2 |
|---|---|---|---|
| US bot match (2648 entries) | 1765 | 755 | 717 |
| EU bot match, rally window (589) | 282 | not run | 12 |

- With section 2, US 888 matches, and so do the other crossfaded switch ticks (237, 243, 371, 391, 411, 426, 427).
- With section 1, the US runs from clip time 6 on (226–236, 249–289, …) match.
- None of the remaining misses are in this spec's scope: see the README.

## Open questions

none

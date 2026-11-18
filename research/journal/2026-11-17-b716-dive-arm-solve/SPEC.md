# B716 spec: the dive's arm solve on the drawn pose

Conventions as in `hst_sim::pose`: row vectors, `A·B` means A applied first (`vmul(a, b)`), a node's model matrix is
its local times its ancestors' locals up to the root, a world matrix is that times the player's placement matrix.
"Rotation part" means rows 0–2 with the translation row zeroed. TIP is the racket point (0, 0.7, 0) in the racket's
frame, as in the contact solve and the dive's arm slide.

## When it runs

Only on a **contact** dive (the dive search found a ball it can reach: N6 *Contact dive*). Miss dives never start
it. It starts on the frame the dive is chosen, right after the dive's slide has been worked out (same frame, same
facing), and it takes over the player's one arm-solve state: whatever contact/volley arm solve was running is
replaced (the stroke start that runs the dive search has already cleared it anyway).

What it sets up:

- ramp length 5, frame counter 0, solve on;
- body step zero and "the body may be moved by the step" **off**: the dive's solve never moves the body (the dive's
  own slide does that);
- the **dive ramp** (below) instead of the stroke ramp, and the **racket turn** on;
- the racket angle (below);
- the four arm turns: right hand none, right forearm none, **right upper arm** the solved turn (below), left upper
  arm none. "None" is the identity rotation, so those three joints are left exactly as animated.

It ends (solve off, dive ramp off, racket turn off) when its ramp runs out, or earlier when anything resets the
arm-solve state: a new stroke/stand/run mode being set, the player being re-placed (point start), or a new contact or
volley solve starting.

## Data it needs (already measured at player load)

At player load the game poses the receive motion (`motion::id::RECEIVE`) at time 5 and keeps, per player:

- the locals of Racket (Lr), Bip01RHand (Lh), Bip01RForearm (Lf) and Bip01RUpperArm (Lu);
- Cu: the model matrix of the right upper arm's parent (identity times each ancestor's local up to the root);
- the racket's model matrix and the right upper arm's model matrix (Lu·Cu): these two are `Reach::dive_arm` already.

All four locals and Cu come from that one load-time pose, not from the current frame's animation.

## The solve (dive start)

Inputs: the player's position Pos, the dive's new facing f (unit, flat, toward the chosen ball), the dive's slide
length, the chosen path frame's ball position B (height as a positive y, the same point the arm slide aims at) and
ball velocity V (y with the same sign convention as B, i.e. the true velocity), and the player's mirror factor h.

1. **Placement.** W = the placement matrix the dive has just set: rows side = (up × f)·h, up = (0, 1, 0), f,
   and Pos as the translation row. T = its rotation part. (This is the same "turn" the dive's arm slide uses; no
   character size scale in it.)
2. **Arm in that facing.** tip = TIP through (racket model matrix · T); U = (upper arm model matrix) · T; the
   shoulder s = U's translation row. Both tip and s are relative to the body's position (exactly the values the arm
   slide computes).
3. **Where the body will be.** P = Pos + f·slide (the end of the slide).
4. **Two directions.** a = unit(B − P − s) (shoulder to ball, 3D); b = unit(tip − s) (shoulder to racket point, 3D).
5. **The upper-arm turn.** axis = unit(b × a); c = a·b clamped to [−1, 1]. R = the rotation about axis whose cosine
   is c and whose sine is +√(1 − c²), built exactly as the contact and volley solves build their turns from a cosine
   (`pose::axis_cos(c, +1.0, axis)`). R carries b onto a.
6. **The solved upper arm.** X's rotation part = (rotation part of U) · R · transpose(rotation part of Cu·W).
   That is the upper arm's local turned by R, R expressed in the upper arm's parent frame.
   Its translation row is never read below.
7. **The stored turn.** q = the quaternion of transpose(rotation part of Lu) · X, converted as the contact solve
   converts its turns (`crate::quat::from_matrix`). This is the right upper arm's turn, slot 3 of the four.
8. **The racket angle.**
   - Cw = rotation part of Lr · Lh · Lf · X · Cu · W: the racket's world frame with the solved arm in the receive
     pose.
   - d = (−Vx, −|Vy|, −Vz): against the ball's travel, with its vertical part always pointing down.
   - d in the racket's frame: e = (d·row0(Cw), d·row1(Cw), d·row2(Cw)) (d times the transpose of Cw).
   - n = unit((e.x, 0, e.z)): flattened onto the racket's local x–z plane.
   - angle = arccos(n.z clamped to [−1, 1]) (the game's `acosf`: the `libm::acosf` the arm table already uses in `pose.rs`). If angle ≥ π/2
     (f32 1.5707964), subtract π (f32 3.1415927).
   - sign = −1 when n.x < 0, else +1 (−0 counts as +1). The racket angle = angle × sign.

   So the racket is to turn about its own y axis until its local z (or −z, whichever is nearer) lines up with the
   incoming ball's flattened direction in the racket's frame.

Degenerate inputs (a parallel to b; d along the racket's local y) give NaN in the game. They need the arm to point
exactly at the ball or the ball to fly exactly along the racket's handle axis; no special case is needed beyond
not panicking.

## Each frame while it runs

The arm-solve block runs once per tick in the player's own update, after that tick's motion step has posed the
nodes and before the racket hand-off (M185 SPEC order), using frame counter n, then n goes up by 1.

**Weight w (the dive ramp)**, with K = the dive's contact frame (the path frame k the dive chose, 5..16):

| n | w |
|---|---|
| 0 … 5 | n / 5 (so 0 on the dive's start frame, 1 at n = 5) |
| 6 … K (including n = K) | 0 |
| K + 1 … | 1 − (n − K) / 5 (0.8, 0.6, 0.4, 0.2) |
| K + 5 | ≤ 0: the solve ends on this tick; nothing is turned |

The first row wins when both apply (K = 5: n = 5 gives 1, n = 6 gives 0.8). The hold at 0 between the ramp up and
the ramp down, and the jump from 0 at n = K to 0.8 at n = K + 1, are the game's behaviour: match them. K stays the
contact frame for the whole dive: the slide being cut short (the dive's own counter jumping to k + 1) doesn't
touch the solve's counter or K. The stroke ramp's rules (the restart at "8 of 8" on reaching 1, the motion speed
going back to 1, moving the body by the step) do not apply to the dive ramp.

**The pose**, in this order:

1. **Racket**, only when w > 0: the Racket node's local becomes Y(racket angle × w) · Lr for its rotation rows,
   keeping the node's current translation row. Y(t) is the rotation by t about (0, 1, 0), built as the arm table
   builds its turns from an angle (`pose::axis_angle(t, [0.0, 1.0, 0.0])`). Note it starts from the load-time
   receive-pose racket local Lr, not from this frame's animated racket local, and turns it about the racket's own
   y axis. When w = 0 the racket keeps its animated local.
2. **Arm**, every tick the solve is on (w = 0 included): right hand, right forearm, right upper arm, left upper arm,
   each turned by weight w of its stored turn exactly as the contact solve's arm turn does (`ArmIk::apply`: slerp
   from the identity by w, multiplied on after the local, translation kept). With the dive's identity turns only the
   right upper arm changes.

Then the hand-off and the M185 racket read see these locals.

In-between frames drawn at a higher frame rate (between two ticks) are presentation only: any weight that
interpolates between the two ticks' weights looks right.

## Where this lands in play

- The Racket world matrix the next player's doubles stand test reads (M185) must be built from the locals after
  step 1–2 above on the diving player's tick; the M185 chain (Racket up through the right arm and spine) already
  covers every node the dive turns except the left upper arm, which only changes the drawing (and is untouched
  anyway: its turn is the identity).
- The drawn skeleton should get the same turns so the diving player's racket and arm look like the original's.

## Open questions

none

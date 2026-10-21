# P133 spec: court prop motions (court 5's ball-boy cart wheels)

Behaviour of the original, in our own words, from reading the game and checking it against the court 5 RAM dump of
P125 (`context/p125/court05.bin`, also copied to `context/p133/court05.bin`). App names used below: `pose::Clip`
(the ANI sampler, N1d1), `pose::Nodes` (node locals + bind), `motion::Clock` (wrap → sample → add speed),
`court::world`/`court::nodes` (collision world and its node walk), `net_hide::buried` (B240's eye-buried test).

## Which props have a motion

- At court load, every placed prop of plant category **19** (structure) and **18** (prop) gets a motion player.
  The motion file is the model's own file name with the extension `ani` instead of `mdl`, in the same directory
  (court 5: `structure/ballboy01.mdl` → `structure/ballboy01.ani`). No file → no clip; the player exists but does
  nothing. Motion number 0 of that file is the one bound.
- Only **category 19** props ever advance their motion (below). A category 18 prop with a motion file stays at its
  bind pose forever. On the disc, the only motion files next to court models are court 5's `ballboy01.ani` and
  court 6's `k-horse.ani` / `k-horse2.ani`, all three in `structure/` (the horses' plant category wasn't checked;
  if they are structures like the carts, they play too). Only the carts collide (P125's scan); the horses are
  drawing only.
- The motion player is the same kind the characters use: looping on, speed 1.0 frame per step, length = the clip's
  length in frames (`ballboy01.ani`: 3200 ticks at 80 ticks a frame = 40 frames).

## Bind (at court load)

Binding resets every node's local matrix to the model's rest local (the disc `node_local`), binds the clip's tracks
to nodes by name, sets the clock to time 0 / sampled 0, and marks "sample on the next step even if the time hasn't
changed". Nothing is sampled at bind: until the first step the wheels are at the disc pose (what `court::world`
builds today).

## The step (once per qualifying court draw pass)

Exactly `motion::Clock::tick` with no hold, plus the sample:

1. wrap the time into the clip (looping: subtract the length while time ≥ length, add it while time < 0; the
   stored time *becomes* the wrapped value);
2. if the wrapped time equals the last sampled time and the "sample anyway" mark is clear, skip the sample;
   otherwise sample the clip at that time into the node locals, remember it as the last sampled time, clear the
   mark, and mark the prop's node tree dirty;
3. time += speed (1.0).

The sample is the character sampler (`pose::Clip`/`pose::Nodes::sample`): a track with more than one rotation key
replaces the node's local rotation rows (translation row kept); a track with more than one position key replaces the
translation (times the track's scale); single-key tracks and unkeyed nodes keep their bind local. For `ballboy01`
that is: tracks `spin_L` and `spin_R` (five rotation keys each, one position key equal to the disc local) turn the
two wheel nodes; nothing else moves.

Because times are whole frames here, the clock runs 0, 1, …, 39, 0, … and the sampled pose repeats every 40 steps.

## When a step happens

The court is drawn once per displayed frame while the court scene is shown. In that pass, structures (category 19)
step their motion only when **all** of these hold for that frame:

- it isn't the first court draw pass after the court's object list was (re)built. The game starts with this
  "first pass" flag set, sets it again whenever a court is torn down, and clears it at the end of every court draw
  pass. So the very first draw of every court is drawn without stepping (and without the visibility tests below);
- the prop is drawn this pass: the visibility tests B240 ported (`net_hide::buried` for objects without the net
  byte; the net-hide test for the one with it) didn't hide it. Those tests only run in the views where the game
  runs them today (see B240/B74d); in other views every prop is drawn. A hidden prop doesn't step that frame;
- the game isn't paused (pause holds every prop clock);
- the global slow-motion timer is not mid-step: when slow motion is active (instant replays' slow shots), only the
  frames whose slow-motion fraction is exactly 1 step; in-between frames draw without stepping.

There is no other gate: cut-aways, the court intro, the post-point flow and the instant replays all draw the court,
so the clock keeps stepping through them. The clock is a count of qualifying *draw passes since the court was built*,
not of match ticks.

## What the ball sees

The node world matrices are recomposed lazily (local · parent down the tree, the walk `court::nodes` already does)
whenever the tree is dirty and someone needs them: at the start of drawing the prop, and at the start of the ball's
sweep test against that prop. In a frame, the game update (ball flight, sweeps) runs before the draw pass. So:

- during a frame's sim update, a ball sweep against a cart sees the wheels at the pose **sampled in the previous
  frame's draw pass** (clock value then, before its +1), recomposed through the cart's still placement;
- a draw pass first recomposes (and draws) the pose sampled in the pass before, then steps.

Checked in the P125 dump (taken between a draw pass and the next update): 13 of the 14 structures' clocks read 516
steps since build (the motion-less ones count too, their time just isn't wrapped; the 14th reads 358 and carries the
"hidden last pass" mark, so it was hidden for 158 passes), the four carts read time 36 with last
sampled 35 (516 mod 40 = 36), their trees are still marked dirty, and the wheel world matrices in RAM are one step
behind the locals (wheel turn 40.32° in the world matrices vs 50.625° in the locals sampled at 35): the world was
composed at the start of that pass from the sample at 34. A ball sweep in the next update would see the sample at 35.

## Quirks to keep

- The motion of category-18 props is bound but never played.
- A hidden cart stops turning while hidden, so the four carts' phases can drift apart only through the visibility
  tests (in practice only if a camera eye sits inside a cart).
- The first court draw pass never steps; pausing never steps; slow-motion in-between frames never step.

## For the port (how to check it)

- Clock: per court instance, steps counted from the court build by the rules above. The remaster has to count the
  same draw passes the original does (intro, cut-aways and replays included) for the wheel phase to line up; with no
  way to know that from match ticks alone, the replay fixtures need the cart clock at their start (or the number of
  passes since the court was built), recorded from the game.
- Recording: on court 5, read the four carts' wheel node matrices (the walk `tests/world.rs`
  `court_5_prop_nodes_match_ram` already uses) every frame through a stretch of play that includes a pause and an
  instant replay, plus the match frame counter. Each frame's wheel matrices must equal sample(t) composed through
  the cart's still nodes, with t following the rules above; P125's dump is a first fixed point (world = sample at
  34, locals = sample at 35, clock 36).
- The collision faces on the wheel nodes then move with the wheels in `mesh` sweeps (the faces are stored per node;
  only the node matrices change).

## Open questions

none

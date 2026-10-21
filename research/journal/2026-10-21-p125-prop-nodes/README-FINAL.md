# P125 — prop collision node walk, off court 10

## Question
`court::nodes` places a prop's child nodes by local · parent (from the disc), checked only on court 10, where every
colliding prop has a single node. Does the game place multi-node props the same way?

## Which courts have them
A scan of `court::world` for courts 1–11: colliding props with more than one node are on court 3 (one prop, faces on
nodes 1–2) and court 5 (four ball-boy carts `ballboy01`, 7 nodes, faces on nodes 1–3: the two wheels and the body;
three balls hang two levels down under node 3). Every court model has many nodes too, but that walk is already the
one the live-ball tests use on court 10.

## Capture
`context/p125/p125_court.py <out.bin>` (under `tools/pcsx2.sh`; journal entries are .md only): loads slot 2 (doubles
character select), ✕ ×4 (P1 Ashley, the COMs' first picks), Start, ✕ on the confirm screen, waits for P1's first
serve (match phase 2), saves scratch slot 9 and writes its EE RAM. The confirm screen's court is random, so each run
lands somewhere else; one landed on court 5 → `context/p125/court05.bin`. I tried steering through Select Court with
right presses: the starting position varies and some presses seemed to get lost. A word I took for the carousel
cursor (0x31106c) read 16 on one run, so it isn't the cursor. Ten runs never reached court 3, so I dropped it (see
below).

## Result (court 5)
The RAM nodes of a prop are 0x120 bytes apart from the instance's first node. `tests/world.rs
court_5_prop_nodes_match_ram`: all 27 colliding props in list order; every node of every prop matches
`court::world`'s matrix bit for bit (16 child nodes, grandchildren included), **except the carts' two wheels**.
Those are animated: `ballboy01.ani` has tracks `spin_L` / `spin_R` turning one full revolution per 3200 ticks
(40 frames at 80 ticks/frame), five rotation keys each, with one position key that equals the disc local. In RAM the
wheels' translation rows match ours bit for bit (the axle is where the walk puts it), but their rotations are
turned. So the walk is right, and the gap is that the prop's own motion isn't played. The wheels carry collision
faces, so a ball hitting a turning wheel sees differently placed triangles in the game.

No other colliding prop has a motion. Court 6's horses (`k-horse.ani`, `k-horse2.ani`) are animated but don't
collide (single node or no collision in the scan). Follow-up **P133**: port the prop motions (clock, rate, loop,
sampling) via spec → port, with the wheels checked against a court 5 recording over frames.

No code change to the walk. The comment on `court::nodes` now says what is verified and what isn't.

## Not verified / not 1:1
- Court 5's cart wheels (nodes 1–2 of the four `ballboy01` props) keep their disc pose in the port. The game turns
  them with `ballboy01.ani`, and they collide. Not ported here because it's a new behaviour (prop motion playback)
  that needs its own spec phase: P133.
- Court 3's multi-node prop wasn't compared. I couldn't get the real game onto court 3: the default court is random
  and the cursor address I found was wrong. It uses the same walk as court 5's carts (whose children and
  grandchildren match), so I didn't open a separate task. Capturing it would need the real court-select cursor in
  RAM.
- Only one moment was compared (first serve). Static nodes can't change, so that's enough for them.

## Round 2 (merge failure)
The first merge failed with "the dump isn't on court 5 (6)". The main checkout's `context/p125/c05.bin` is a court 6
capture (an earlier run of the script); the real court 5 dump was only in a runner slot's `context/` (written a
minute later), and context sync never replaces a file the main checkout already has. The test now reads
`context/p125/court05.bin`, a copy of the court 5 dump under a name nothing else uses, and passes. The follow-up for
the cart wheels is **P133**: the id P127 the first round used was taken by another task in the meantime.

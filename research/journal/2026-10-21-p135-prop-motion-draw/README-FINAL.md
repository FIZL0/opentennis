# P135: drawing court prop motions

## What's in
- `hst_sim::court::drawn(iso, n, season)`: every placed structure (category 19) whose model has a `.ani` next to it,
  colliding or not, keyed by its plant record. Each gets its own `court::Motions` (one `PropMotion`, root identity, so
  `world_nodes` is in model space; the disc roots are identity and the unsampled walk equals `node_bind`).
  `Motions::draw_pass` is `pass` without a collision world. Both use the same clock and gates as P133.
- **The horses' plant category**: all five of court 6's horses (3 × `k-horse`, 2 × `k-horse2`) are category 19
  structures in both seasons, so per the P133 spec they play their motion. Court 5 has 4 `ballboy01` carts
  (category 19; `ballboy02` has no motion).
- `hst_data::mdl::Model::skin_entries`: a skinned packet's drawn vertices as their (node, pre-weighted position,
  weight) entries, in `skin_packet`'s order.
- `stage.rs`: `batch_key` (the court mesh batching key, now shared with `main.rs`), `spawn_posed` (per-instance copies
  of the model's meshes, each vertex's sources, an Aabb over every frame of the motion), the `PropPose` component and
  `step_props`. That runs once a FixedUpdate tick, not while paused. Slow motion's in-between frames run no tick, so
  they don't step. A prop the eye tests hide (`Visibility::Hidden`) holds. The first pass doesn't step. Moved
  vertices go through `vertex_patch`.
- `main.rs` `load_court` spawns those plant records through `spawn_posed` instead of `spawn_parts`.

## Checks
- `stage::tests::posed_props_start_at_the_drawn_mesh_and_move`: court 5 has 4 motions and court 6 has 5. For every
  vertex of the carts' and both horse models, the posed position at bind is where `mesh` drew it (< 1e-4), and
  half-way through the motion some vertices have moved.
- Shots (`context/p135/`, `HST_EYE` fixed views): `c6_2.0.png`/`c6_2.25.png` show the horses intact, with heads and
  tails in different poses between the two frames. `c5.png` shows the cart intact.

## Not verified / not 1:1
- Not compared side by side with the original game. I didn't capture court 6's horses or court 5's carts in PCSX2:
  getting the game onto those courts takes P133's Select Court save dance. The poses come from the disc motion
  through the same sampler as P133's bit-exact cart check, so only the look (rendering) is unchecked. The brown
  horse's neck stretches as it raises its head; that is the skinning of the disc motion, which I didn't check
  against the original.
- Normals aren't re-posed (`vertex_patch` writes positions only). Lighting on the turning wheels and the horses'
  moving parts uses the bind-pose normals.
- The clock counts match ticks (FixedUpdate), not the original's draw passes. The instant replay's unseen skip
  ticks step it, where the game doesn't draw those frames. The drawn carts aren't tied to the colliding world's
  `Motions` either: that is P134's job (in the match, snapshot). The phase is presentation only.

# B676b spec: the thigh twist blend

This describes what the original does. I wrote it from the decompile and checked it live over PINE against the drawn
thigh matrices of 8 characters in user save states 3, 4 and 5 (`scripts/b676b_thigh.py`). The rebuilt matrix matched
the game's within 1e-6 for every thigh. The Port phase works from this file.

## What it is

B676's `extra_turns` probe showed each thigh and calf "bent" by the same angle (7–27°). That is not a bend. It is one
change, made to the **thigh only**, and it touches only the matrix the thigh's own vertices are skinned with:

- The thigh keeps its animated **bone axis** (the direction the bone points down the leg). It also keeps its
  animated position.
- Its **roll about that axis** (the twist) is pulled part of the way back toward the model's rest twist. The share
  that stays animated is a per-character, per-costume factor.
- The calf, foot and toes are **not** moved. Their drawn matrices come from the unchanged, animated thigh. That's
  why the probe saw the calf "turn" by the same angle as the thigh. Relative to the changed thigh, the calf's frame
  is turned back by exactly the amount the thigh was rolled.

So the effect is a skinning fix: on these characters the thigh's mesh twists half as much as the animation turns
it, and the knee, shin and feet stay exactly where the clip puts them.

It does not feed gameplay. Nothing else in the game reads the thigh's adjusted matrix. Body-hit uses the head, spine
and neck, and footstep dust uses the feet, which this doesn't touch. It falls under the rendering rule ("looks
right"), but it's cheap to make it exact, and the rule below is exact.

## When it runs

- Once per frame per player, in the pose-finalize step. It comes after every node's drawn (world) matrix has been
  worked out from the sampled local pose, after the motion crossfade and arm IK, and before the head look (B675,
  which touches only the head).
- It runs for both thighs (right, then left; the order doesn't matter, they don't depend on each other).
- It's skipped entirely when the player's twist factor is exactly 1.0. The drawn thigh is then the plain animated one.
- It's idempotent: it always starts from the thigh's animated local matrix and the parent's drawn matrix, never from
  its own previous output.

## The twist factor

The factor `s` depends on the character (index 0–13, the same numbering as the `PCnn` files) and on the costume
group: costumes 0–4 use the first value and costumes 5–9 the second.

| character | costumes 0–4 | costumes 5–9 |
|---|---|---|
| 0 | 0.5 | 0.5 |
| 1 | 1.0 | 1.0 |
| 2 | 0.5 | 0.5 |
| 3 | 1.0 | 1.0 |
| 4 | 0.5 | 0.5 |
| 5 | 1.0 | 1.0 |
| 6 | 1.0 | 0.5 |
| 7 | 1.0 | 1.0 |
| 8 | 1.0 | 1.0 |
| 9 | 1.0 | 1.0 |
| 10 | 0.5 | 0.5 |
| 11 | 1.0 | 1.0 |
| 12 | 0.5 | 0.5 |
| 13 | 1.0 | 1.0 |

The factor is set when the player's character and costume are set up, and it doesn't change during the match.
I checked it live: characters 0, 2, 4 and 6 in costume 9 use 0.5, and characters 1, 3, 5 and 11 use 1.0.

## The rule

The inputs, per thigh:

- `L`: the thigh's animated local matrix (row-vector convention like the rest of our pose code). Rows 0–2 are the
  rotation rows, row 0 is the bone axis, and row 3 is the translation. It's the same local the drawn pose already
  uses (clip sample, after any crossfade).
- `R`: the thigh's rest local matrix from the body model (the node's bind local, what `Skeleton::rest` holds). Only
  its three rotation rows are used.
- `P`: the parent's (pelvis's) drawn world matrix.
- `s`: the factor above.

Steps:

1. `a` = R's row 0, normalised (the rest bone axis).
2. `b` = the cross product a × (L's row 0), normalised. This is the turn axis.
3. `c` = b × a, normalised. It's the direction in the a–L₀ plane perpendicular to a, on L₀'s side.
4. Turn angle: its cosine is `L₀ · a` clamped to [−1, 1], and its sine is `+sqrt(1 − cos²)`. The sine is only
   negative if `L₀ · c` < 0, which can't happen with c built as above. So this is the shortest-arc turn about b that
   carries a onto L₀'s direction.
5. Apply that turn to each of R's rotation rows: express the row in the (a, c, b) basis, rotate its (a, c)
   components by the angle (a goes toward c), and go back. Call the results `T₀, T₁, T₂`. T₀ is R's row 0 laid onto
   L₀'s direction, with R's row-0 length.
6. The new local matrix:
   - row 0 = T₀
   - row 1 = normalise(s · L₁ + (1 − s) · T₁)
   - row 2 = normalise(s · L₂ + (1 − s) · T₂)
   - row 3 = L's translation (unchanged)
7. The thigh's drawn world = new local × P. Only the thigh's own drawn matrix is replaced. The children keep the
   world matrices they got from the unchanged thigh.

With s = 0.5 that's the halfway twist between the animated and the rest-carried roll. Because rows 1 and 2 of L
and of T are both perpendicular to the same axis, the blended rows stay perpendicular, and normalising each on its
own gives an orthonormal frame. (The rest rows of the disc skeletons are unit length, so T₀ is L₀'s direction at
unit length.)

### Degenerate case

If L₀ points exactly along the rest axis (a × L₀ = 0), the original's vector unit makes b and c zero instead of
NaN. The turn is then the identity, and rows 1 and 2 of T collapse to zero. So the blended rows come out as L's own
rows 1 and 2, and the thigh is drawn exactly as animated. The port should do the same: no change in that case.
This can only happen when the thigh's axis matches the rest pose exactly; I never saw it.

## Measured (live, game vs this rule)

Twist removed (the angle between the plain and the drawn thigh frame), per right/left thigh:

| state | player | twist R / L |
|---|---|---|
| slot 3, Carol (char 6, costume 9), serve stance | factor 0.5 | 27.0° / 21.8° |
| slot 4, Carol (char 6, costume 9), ready stance | 0.5 | 5–7° / 5–7° |
| slot 4, char 4 (costume 0) | 0.5 | 8.0° / 7.9° |
| slot 5, char 0 | 0.5 | 27.1° / 21.4° |
| slot 5, char 2 | 0.5 | ~10° / ~7.6° |
| chars 1, 3, 5, 11 | 1.0 | 0 (skipped) |

The rebuilt drawn thigh matched the game's within 1e-6 (float64 rule against the game's float32 matrices) for every
thigh with factor 0.5.

## Port notes (not behaviour)

- In our renderer the joints form a transform hierarchy, so turning the thigh joint would carry the calf with it. To
  change only the thigh's skin, either give the thigh's skinning matrix its own value, or turn the thigh and give
  the calf the inverse turn in its local. That second way is exactly what the game's numbers look like, and it's what
  `character.rs` already does for a mod's feet ("drawn turned by its bind's conjugate of the turn back"). Anything
  that reads the calf, foot or toe joints (feet on the ground, dust, shadows) must keep seeing the animated chain.
- Mods: a mod character's factor is ours to choose, because additions are layered on top. Stock characters must use
  the table. One suggestion is 1.0 (off) for mods unless their manifest asks for it, since a mod's thigh skin weights
  aren't the original's.
- Front-end previews (character select, inspect) are ours to design. Using the same rule there keeps the legs
  looking the same as in a match.

## Open questions

none

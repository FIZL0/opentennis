# B579 — Sophie's pigtails have no physics again

## Symptom
In a match, the Out of Bounds Sophie mod's pigtails held one shape and didn't swing (user 2026-10-19).

## Cause
Nothing was disconnected. The mod still has its 32 spring joints, the system runs, and it steps at 60 Hz.
A per-step log of the solver in a CPU match (`--slot-mod 0 …/oob_pc02_sophie`) showed what was wrong: the upper
pigtail links sat **exactly at their angle limits** every step (5°, 30°, then ~18–20°, std 0.00–0.16°). The ribbons'
lower links did the same, so only the very tips twitched. Two parts of B410b combined to cause this:

1. `SOFT` (×0.1) softened the pull back to the pose but not gravity. The rest sag became 10× what OOB's own
   stiffness:gravity balance gives: ~2.7 cm on a 4 cm link, beyond every link's limit. Each link rested pinned
   against its limit.
2. Only a chain's first link kept world inertia. Every link below rode rigidly with the link above. The pigtail's
   first link has a 5° limit, and it was pinned too.

B410b was picked side by side before the `HANG` follow-up. Back then, a rig placed more than 1 m away (each point's
set-up) restarted from the bind pose, and the hair visibly fell for ~2 s every point. That fall was most of the
movement anyone saw. `HANG` (fresh rigs start already hung) removed it, and the pinned steady state was all that was
left: the "regression".

## Fix (`crates/hst-base/src/spring.rs`)
- Gravity ×`SAG` = 0.3. Every link now rests inside its limit, and the hang still looks like B410b's (viewer shots
  `context/b579/run1/view_a.png` before, `view_e.png` after; ×0.1, `view_d.png`, held the pigtails out sideways).
- Every link keeps its world motion, capped by `LAG` (3° a step), not just the first. The `Mass::first` flag is gone.

Match log, Sophie's pigtail links (std of the angle off the pose / mean change per step):

| | upper links | mid links | ribbons |
|---|---|---|---|
| before | 0.00 / 0.0° | 0.1–0.7 / ≤0.1° | 0–4.9 / ≤1° |
| after | 8.0 / 1.8° | 4.4–5.7 / 1.3–2.4° | 2–10 / 0.5–2.6° |

Variants tried (KAMI chain test, sprint sway / idle jitter per step): gravity ×1 + all links → idle jitter 2.96°
(B410b's original whip, 2.99°); ×0.1 + all links → 1.29° but the pigtails stick out; ×0.3 + all links → 30° sway,
1.5° jitter (kept); gravity ×0.1 alone → 0.8° sway (still stiff).

## Tests
- `spring::tests::a_sprint_swings_the_pigtail` (new): Sophie's chain on a 3 m/s sprint and stop has to sway >10°,
  settle, and keep idle jitter <2°/step. On the old solver it fails with 0.09° sway.
- `mods::tests::oob_hair_swings_and_settles` now runs Sophie's own mod too (32 springs) next to Jasmine.

## Not verified / not 1:1
- Out of Bounds' own solver is still not ported (as in B410b). `SAG` = 0.3 was picked by eye in the viewer and from
  the match log, without the user. The hang is close to B410b's but not identical (less sag at the chain ends).
- No side-by-side video with the user. The movement was judged from per-step logs and stills, not by watching it.
- Other OOB mods' chains (Jasmine's earrings, scarves) change too. The test checks that Jasmine still trails and
  settles; nobody has looked at them in a match.

# M132: golf mod hair swings more (picked with the user)

Reference: the Out of Bounds video named in M130 (Sophie's animations, 60 fps). On spins and swings her pigtails fly
out almost horizontally and then fall back to hanging.

**Ours before:** only a chain's first link kept its world motion (capped at `LAG` = 3° a step). Every link below rode
rigidly with the one above (B410b), so mid-swing the pigtail just hung. `a_jump_bends_the_lower_links`: link 3 bent
1.2° on a 30 cm head jump.

**Change (`crates/hst-base/src/spring.rs`):** lower links keep their world motion as well, with the lag growing by at
most `CHILD_LAG` = 2° a step. At rest the hang is unchanged. The match clips at the same frames show the same hang
whenever she stands still.

**Picked from clips:** CPU match, `HST_WEATHER_SEED=1234`, the same rally each run (deterministic: only hair pixels
differ). The camera follows Sophie (`HST_FOLLOW=3.0,3.14,0.95`, `HST_CLIP=600`, `--shot-at 20`; scripts in
`context/m132/bin/`). The user chose F out of:

| | first-link cap | lower-link cap | |
|---|---|---|---|
| A | 3° | 0 (rigid) | before |
| B–D | 3–8° | ×0.5–1 of the world motion, uncapped | flung out, but puffed out and curled up while idle: rejected |
| E | 3° | 1° | a little more |
| **F** | 3° | 2° | swings out like OOB, hangs at rest |
| G | 5° | 1.5° | close to F |

Clips: `context/m132/run5/pick.mp4` (A/E/F/G grid), `oob_ref.mp4`.

**Tests:** new `a_jump_bends_the_lower_links` (fails with `CHILD_LAG` = 0). `a_bobbing_head_does_not_whip` raised from
5° to 8° a step (F gives 5.9°; the B410b bug it guards against was 38°). `oob_hair_swings_and_settles` passes.

**Not done:** OOB's own solver is still not ported, and its collision ellipsoids are not used. Other OOB mods' chains
(Jasmine's earrings, scarves) change too. They were not looked at in a match.

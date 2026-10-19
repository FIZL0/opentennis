# M130 — Sophie's missing animations

Task: the mod used only a few of the character's own animations; add the missing ones and bump the mod's version.
User steer mid-task: find the animations in the source game's data rather than the video; leave `re_ball` to M131.
No game or mod content is recorded here or in git (user, 2026-10-19): the mods repo keeps only how takes are picked
when building from the user's discs.

## Finding
The source extraction already holds every animation the reference shows. The gap was HST's side: a mod could give
each of the four reactions only one clip of its own, so most of the character's emotes and result poses went unused.

## What was added
**Takes**: a mod's `motions.glb` may carry `NAME.1`, `NAME.2`, … beside `NAME`; each time the motion starts one of
them is drawn in turn (`character::take` on `Motion::sets`, which the rollback snapshot already carries; no match RNG
touched). Loaded in `mods.rs` `own_motions` (a take without its motion, or a suffix that isn't a whole number ≥ 1,
fails the load), drawn in `character.rs` (`CharacterData::takes`, `drawn_take`, `own`). It works for every motion and
every mod, so M131 can use it.

Mods repo: standard §6 documents `NAME.K`; `tools/rerig/takes.py` + `package.py` build the takes from the extracted
clips (picked by face-channel mood and a camera-facing start). The mod was rebuilt with takes on all four reactions,
`mod.json` version 3 → 4.

## Tests
- `mods::tests::takes_load_and_take_turns`: load order, the two load errors, the pick reaches every take.
- `mods::tests::a_mods_takes_load_beside_its_motions`: the first installed mod with takes loads them, none shorter
  than the donor's clip.
- Full suite: everything passes except `mods::own_motions_are_visual_only` and `mods::golf_faces_all`. Both come from
  M131's rebuild of another mod in the shared `out/mods` and are noted on M131's TODO line; this change doesn't cause them.

## Not verified / not 1:1
- Nothing compared frame by frame with the reference video. Clips were judged by eye only, as the user asked.
- No in-game shot of the takes playing (unattended run). The check is the load test above.
- Cameras that frame a reaction (cut-away, replay) still use take 0's length.
- Menu previews (inspect) go through `Motion::set`, so they show takes too.
- Not re-uploaded to the mod store; version 4 is ready for the next upload.

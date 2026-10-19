# M131: more celebration and anger takes for the other golf mods

Task: as M130 did for Out of Bounds Sophie, look for each other Hot Shots Golf mod's own animations and add the
missing ones. User, mid-task: the ball-retrieve motion stays as it is; the point is extra **victory celebrations,
anger and such**. Also mid-task: **drop Robot Tennis** as a mod character and take it off the store.

## What the golf characters have that the mods didn't use

The point/game reactions (re_gu, re_di, re_gu_set, re_di_set) each had one golf clip. Every character has more
clips in the same moods, and each game names them the same way:

- Out of Bounds: the emote set `ku_{gl,ha,cl,sh,an,su,aw,…}` (glad, wave, clap, hands to cheeks / angry stamp, sulk),
  hole results `ga_{bi_r00,bi_r01,ea,pa,bo}`, results-screen poses `rslt_{win,vict,get,loss,loss_2,clap,wait}`.
  The shot reactions `re_pc_p/wi` start turned 19–81° towards the ball, and `re_pc_ga` sits down while turned, so
  neither is used.
- Fore!: `re_{tp,lp}` (fist pump, hand to face), `re_{ga,os,bw}` (shrug, sits/kneels holding the head, bent over
  on the knees), `ga_*` hole results, `win/lost/clap/up` results. `up` raises the character out of frame and isn't
  used. Some Fore! characters have face channels, most don't.
- Open Tee 2: `ku_{gl,ha,cl,sh,an,di,aw}`, `re_{da,os,…}` (stomp, point), `ga_*`, `win/lost`. There are no face
  channels. `ku_sh` here reads as hands to the head (dismay), so it isn't used.

## What was added

M130's mechanism: a mod's `motions.glb` holds extra clips named `re_gu.1`, `re_gu.2`, … and the game draws one in
turn each time the reaction starts. The mods repo's `tools/rerig/takes_pick.py` builds the table in `takes.py` for
every golf mod except Sophie, who keeps M130's hand-picked entry. Each game has a candidate list per reaction (by
the clip's name and its meaning), minus the reaction's own clip. A candidate is kept only if all of these hold:
- It starts within 45° of the character's median start facing. Some characters stand slightly turned in every
  pose; for example LJ's emotes are all at −41°, and the render shows him facing the camera.
- It ends within 60° (spins that finish facing away are dropped).
- Where there is a face, its mood (smile/laugh vs anger/sorrow/vexing/sad) is not the opposite of the reaction's.

On Sophie, these rules give exactly M130's hand picks. 119 candidates are left out, mostly `ga_bi_r01` spins
(25), Fore! `re_bw` (starts side-on, 24) and turned eagle/par/bogey clips. `takes_pick.py` prints the reason for
each one to stderr.

Result: 544 takes over 62 mods (61 new). All three games were repackaged (this also cleared re_ball clips that an
earlier, dropped experiment had left in out/), and the version of each of the 61 mod.json files was bumped. Sophie
was not bumped; she is M130's. Mods repo commit a438bb1, pushed.

Takes per mod (re_gu / re_di / re_gu_set / re_di_set):

| mod | re_gu | re_di | re_gu_set | re_di_set |
|---|---|---|---|---|
| oob pc02 | 4 | 2 | 6 | 2 |
| oob pc00 | 4 | 2 | 6 | 1 |
| oob pc01 | 4 | 2 | 4 | 2 |
| oob pc03 | 4 | 2 | 6 | 2 |
| oob pc04 | 4 | 2 | 5 | 2 |
| oob pc05 | 2 | 2 | 1 | 0 |
| oob pc06 | 4 | 2 | 4 | 2 |
| oob pc07 | 2 | 2 | 3 | 2 |
| oob pc08 | 3 | 2 | 1 | 2 |
| oob pc09 | 4 | 2 | 5 | 2 |
| oob pc10 | 4 | 2 | 5 | 0 |
| oob pc11 | 4 | 2 | 1 | 2 |
| oob pc12 | 3 | 2 | 6 | 2 |
| oob pc13 | 4 | 2 | 4 | 1 |
| oob pc14 | 4 | 1 | 5 | 2 |
| oob pc15 | 4 | 2 | 6 | 2 |
| oob pc16 | 4 | 2 | 5 | 2 |
| fore pc00 | 1 | 1 | 5 | 1 |
| fore pc01 | 1 | 1 | 4 | 1 |
| fore pc02 | 1 | 1 | 3 | 1 |
| fore pc03 | 1 | 1 | 5 | 1 |
| fore pc04 | 1 | 1 | 5 | 1 |
| fore pc05 | 0 | 1 | 4 | 0 |
| fore pc06 | 1 | 1 | 4 | 1 |
| fore pc07 | 1 | 1 | 5 | 0 |
| fore pc08 | 1 | 1 | 5 | 0 |
| fore pc09 | 1 | 1 | 5 | 1 |
| fore pc10 | 1 | 1 | 5 | 1 |
| fore pc11 | 1 | 1 | 5 | 1 |
| fore pc12 | 0 | 0 | 4 | 0 |
| fore pc13 | 1 | 1 | 5 | 1 |
| fore pc14 | 1 | 1 | 5 | 1 |
| fore pc15 | 1 | 1 | 5 | 1 |
| fore pc16 | 1 | 1 | 2 | 1 |
| fore pc17 | 1 | 0 | 4 | 1 |
| fore pc18 | 1 | 1 | 3 | 1 |
| fore pc19 | 1 | 1 | 3 | 0 |
| fore pc20 | 1 | 1 | 3 | 1 |
| fore pc21 | 0 | 1 | 5 | 0 |
| fore pc22 | 1 | 1 | 5 | 0 |
| fore pc23 | 1 | 1 | 5 | 1 |
| opentee2 pc00 | 2 | 3 | 4 | 1 |
| opentee2 pc01 | 2 | 3 | 3 | 1 |
| opentee2 pc02 | 2 | 3 | 2 | 1 |
| opentee2 pc03 | 2 | 3 | 3 | 1 |
| opentee2 pc04 | 2 | 3 | 4 | 1 |
| opentee2 pc05 | 2 | 3 | 4 | 1 |
| opentee2 pc06 | 2 | 3 | 3 | 1 |
| opentee2 pc07 | 2 | 2 | 2 | 1 |
| opentee2 pc08 | 2 | 3 | 4 | 1 |
| opentee2 pc09 | 2 | 2 | 4 | 1 |
| opentee2 pc10 | 2 | 3 | 3 | 1 |
| opentee2 pc11 | 2 | 3 | 4 | 1 |
| opentee2 pc12 | 2 | 3 | 4 | 1 |
| opentee2 pc13 | 2 | 3 | 1 | 0 |
| opentee2 pc14 | 2 | 3 | 3 | 1 |
| opentee2 pc15 | 2 | 3 | 4 | 1 |
| opentee2 pc16 | 2 | 3 | 2 | 1 |
| opentee2 pc17 | 2 | 3 | 4 | 1 |
| opentee2 pc18 | 2 | 3 | 1 | 0 |
| opentee2 pc19 | 2 | 3 | 2 | 0 |
| opentee2 pc20 | 2 | 3 | 2 | 1 |

## Robot Tennis

- opentennis: `mods::DROPPED` gains `hst_umpire_03_robottennis`, so the custom roster and the store list hide it
  even when it's installed. The modstore test now expects only Suzuki. mod_match's umpire rally test became
  `umpires_stay_off_the_roster`.
- Mods repo: `hst_umpires.py` no longer builds it (f2714c5); its out/mods folder was deleted.
- HF store: removed from PR #151 (commit 4c71dc6). main's index.json lists it until the hourly index job
  reruns; the game hides it anyway.

## Not verified / not 1:1

- Not looked at in a match. The picks were checked with retarget renders (sample sheets: Phoebe, Mel, Mika, LJ)
  and the clip names in motions.glb, not in-game.
- The 61 rebuilt mods were not re-uploaded to the store (unattended run). The versions are bumped and ready
  for the next upload.
- Fore! characters without faces and all of Open Tee 2 were judged by clip name and a handful of renders, not by
  face mood. The facing limits (45°/60°) and the face threshold (0.3) are my choices.
- Takes are timed like the reaction they stand in for (M130): a longer clip like Fore!'s 200-frame hole results is
  cut off or held by the game, not shortened.

## Tests: blocked on M130

- This branch's game code rejects `re_gu.1` ("is not a motion name"). M130's game side (13f1db3f on task/M130, not
  merged yet) reads takes, and every checkout's `mods/` links the shared out/mods. So until M130 merges, 10 mods
  tests fail here. Sophie's M130 build alone already triggers them.
- Run on M130's worktree: 27 of 28 mods tests pass. `own_motions_are_visual_only` maps every motions.glb clip
  name to a motion and doesn't expect takes on Fore! Phoebe; that test needs to skip `NAME.K` (noted on M130).
- That run also caught a real regression, now fixed: repackaging from out/rerig dropped the Fore! face stand-ins
  that face_alias.py (B561) had added to the packaged costumes in place (Phoebe's doki_mouth). I re-applied them to
  all golf mods, and package.py now applies them itself (mods df27aaa).

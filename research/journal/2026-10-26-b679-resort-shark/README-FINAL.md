# B679 — the resort court's shark floats on top of the sea

## Which shark

The task line pointed at the disc's court 2 creature `trg_SA-04`. That one is a crab (trigger type 5): it stays hidden
until a ball startles it, its home is in the surf, and nothing about it floats. Court 2 on the disc has no shark (its
other trigger, `M-02`, is the airplane flyover). The user then pointed to the right one: the shark is in the **Get a Grip
resort court mod** (`gag_c04_resort`, "Laguna Beach Resort Court"), its `gimmick.glb` instance `gi09_Shark_r6`. The disc
crab and the match sim are untouched.

## Why it floated

The mods repo's exporter (`tools/psp2gltf/export_getagrip_courts.py`) puts each escape/break gimmick at the first point
of its route in `c04_route.dat` and loops its resting motion. The shark's route (6, ten points looping 78–118 m off the
far baseline) is at **y = +1.28 m**. The sea there (`c04_resort_water01@water`) is at **−1.31 m** (an additive glint
layer at −1.25) and the seabed at −3.24 m. The posed shark (the Swim motion scales its root joint to 0.6 and lowers
it) reaches from 2.39 m below its root to 0.95 m above it. So the whole shark, belly included, sat above the water.

The Swim motion has no root/height track (checked every track of `c04_gi09_Shark_Swim.i3m`: joint1 translation is
constant, joint6 has a constant 0.6 scale and offset). The PSP program, which is encrypted, must hold the swimmer at
the water, and the data doesn't say how deep. Other routes are in the court's own heights: the six crabs' routes are
at y = 0, on the sand at 0.

## Fix (mods repo, bb58b14, pushed)

A new `sea_level(ground model)`: for a gimmick's route point, if the first ground surface below it is water (a
`water` texture), the gimmick is placed at the sea's height (`@water`) instead. With its root on the sea, the
shark's back is just under the surface and only the dorsal fin and tail tip show. Its belly touches the seabed, and
its lower fins go a little into it, under the water. On every Get a Grip court this moves only c04's shark and one
surfboard, route 8 off the pier. That board was also 1.3 m above the sea. Now it's on the water and no longer
between the match eye and the court, so it's no longer `__clear`-hidden. The crabs and the marlin don't change, and
no other court has a point over water. The manifest's `gimmicks` list says "on the sea" for both.

`gag_c04_resort` was re-exported. Its `mod.json` version is already 2 (bumped by B678's sea scroll, not uploaded
yet; the store has 1), so this rides in the same v2 upload.

Checked in our renderer (`tools/shot.sh --play --court-mod …/gag_c04_resort`, `HST_EYE` side view and the match
camera; shots in `context/b679/fix/`): the shark swims in the sea with its fin out; from the match camera it shows
in the water past the far baseline.

No game code changed: no RELEASE_NOTES line (pure mod update), replay tests unaffected.

## Not verified / not 1:1

- Its depth is by rule, not from data: Get a Grip's encrypted program places the swimmer, and its data has no depth.
  It wasn't compared with Get a Grip running (no PSP emulator setup here).
- It still loops Swim in place at its first route point. Swimming the route loop and the escape reaction are the
  program's, not reproduced (same as every gimmick, listed in the manifest's `todo`).
- The blue marlin (route 7, y 2.08 m) sits over ground at +4.44 m (no water under it), so it may be inside rock.
  Not looked into: not part of this report.
- The re-exported mod still needs uploading to the store (user's step, with B678's v2).

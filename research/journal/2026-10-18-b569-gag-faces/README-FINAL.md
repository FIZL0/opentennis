# B569 — Get a Grip mod faces too zoomed in

## Cause
Not the rigs. Every mod costume baked on the device is framed on its `Bip01Head` bone (B176: square = 1.6 × hair top
to bone); those frame Get a Grip like the disc (`context/b569/pair_getagrip.png`: costume 0 left, costume 1 right).
But costume 0 of a **store-installed** mod is not drawn locally (B335): its card is the listing's `preview.png` and
its face close-up is cut from that picture by `face_square(img, None)`, B133's old picture-only path with a fixed
100-px square (384×480 drawing). Bone-framed squares measured from the cache (`truth.py`: alpha template match of
each `-face.png` inside its drawing):

| set | bone-framed square (px) |
|---|---|
| disc costumes | ~127 (114–174) |
| Fore / OOB costume 1 | ~125 |
| Get a Grip costume 1 | ~148 (114–186) |

So every store mod's icon was ~80 % of the bone framing, Get a Grip's ~68 % (big chibi heads): chin cut off, head
filling the tile.

## Fix
`mods::head_height(m, costume)`: the costume's rest-pose reach above `Bip01Head` (highest vertex on the head joint or
a joint under it; skinned vertices by their heaviest joint; PSP heads' rigid mesh nodes under the bone), read from
the glb alone (cheap, no textures, works from modfs in the browser). `from_store` sizes the picture's square from it:
`face_square_sized(img, HEAD_BOW × metre_px × h)`: 1.6 × that height like the bone, top 7 % of the side above the
hair like the bone; centre as B133's picture path. `metre_px` = FOCAL.y / 8 m × size.y / widened crop height
(215.7 px/m at 384×480). `HEAD_BOW` = 0.96: median of bone-framed square / (1.6 × 215.7 × rest height) over 57 store
mods' costume 1 (`chin2.py` + `truth.py`); ±4 % for most, a few up to 25 % looser (Chaos's spikes, Sam's hat,
OOB Jasmine's ponytail stand up at rest but sit off the bone's ±12 px column in the drawing). Looser, never cut.
`CUT` salt in the mod cache stamp so every installed mod's card/face is made again once.

Checked: python reimplementation on every store mod's picture vs its bone-framed costume 1 (`context/b569/new_*.png`),
then the real build (`HST_MENU=mods --shot`, cache beside the worktree's ISO): all 15 Get a Grip faces and every
other pack's costume-0 face beside disc ones (`context/b569/game_new.png`). Test `head_height_over_the_bone`
(Emi, Suzuki, Phoebe vs the numpy sum).

## Not verified / not 1:1
- Not a match to the original: front-end face icons are ours (PLAN: menus aren't 1:1); compared against our own
  bone-framed bakes, by eye and by square size.
- A few store mods with hair/hats standing up at rest come out up to ~25 % looser than a bone-framed bake (above).
- A mod that ships its own `previews/*.png` still gets B133's fixed 100-px cut (`face_from` → `face_icon`): none of
  the installed packs ship one, so it wasn't changed.
- Disc framing (incl. Ashley's B219 zoom) untouched.

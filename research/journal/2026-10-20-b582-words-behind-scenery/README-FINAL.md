# B582 — ball-hit sound words drawn behind scenery

## Cause
The bounce words (court/net/wall contacts: "bumps", "rattle", …) and the body-hit CONK/SMACK are camera-facing
blended quads (`play/bodyhit.rs`) with no sort offset. Bevy sorts blended draws by the mesh's translation plus
`depth_bias`; the court's blended scenery draws with order 0 and sorts by its own mesh centre, so any big
see-through scenery mesh whose centre was nearer the camera than the word drew after it and painted over it
(the same cause as B75, which only moved the backdrops first). The player markers and serve balloons already
avoid it with `markers::LAST`.

## Fix
Both word materials get `depth_bias: markers::LAST`: they sort after every scene draw (still Z-tested, no Z
write, as the markers), below the umpire's calls (`CALL_SORT`).

## Checked
- `tools/check.sh -p hst-play bodyhit` builds; release build runs (`tools/shot.sh` court 1/5/10 with
  `HST_BODY_HIT=1 HST_AUTOPLAY=1`, context/b582/): the SMACK pop draws normally.

## Not verified / not 1:1
- No before/after pair showing the bug itself: bot runs aren't repeatable frame for frame and none of the shots put
  a word in front of nearer blended scenery; the cause is from the sort keys (scenery order 0 by centre vs. words
  with none).
- Whether the original Z-tests these words wasn't read from a GS dump; kept Z-tested like the markers (a word can
  still be covered by a player's body or a wall actually in front of it).

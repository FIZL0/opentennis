# B712: a prototype disc beside the release discs

`discs::tests::finds_the_other_releases` failed after another task put a European prototype image
(`SCES-54535-proto-0119.iso`) into the shared `fixtures/discs/`. Its file name sorts before `SCES-54535.iso` and
`beside()` kept the first copy of a release it found, so it returned the prototype.

## Done
- `beside()` no longer keeps simply the first copy. Of two copies of one release it keeps the one ranked more surely
  retail: a pack whose hash is a known retail copy (`KNOWN_PACKS`) first, then an ISO by its primary volume
  descriptor's creation date (a prototype is mastered before the shipped disc: proto 2006-09-15, retail 2007-02-14),
  then the first one found. It reads one sector per candidate copy, not the whole disc.
- The test now links the discs it needs into a temp folder of its own, so other files dropped into
  `fixtures/discs/` can't break it. When the prototype is there, it also checks that the retail disc wins over the
  prototype, whether the prototype comes first in the same folder or in an earlier folder.
- The prototype stays in `fixtures/discs/`. It's shared with the main checkout and probably another task's input, so
  it wasn't moved.

## Not verified / not 1:1
- none (front-end/disc handling, not gameplay). A pack with an unknown hash ranks below any ISO with a date, and a
  web-mounted image (no file on disk) has no date, so it falls back to first-found. Both are only a tiebreak between
  two copies of the same release.

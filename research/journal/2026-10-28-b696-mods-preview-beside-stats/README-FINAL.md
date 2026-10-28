# B696 — Mods page: a character's picture beside its stats card

**Change** (`mods_screen/grid.rs`, `detail`): for a character with room for its stats card, the picture comes out of
the coloured title strip and sits on the left of the stats card in a light tile, as the character select's cards
put the thumbnail beside the grades. The strip keeps only the name and badge and gets shorter (`STRIP_MIN` up to
`ASIDE_STRIP` 84 px). The picture row takes the room first, up to `ASIDE_ROW` 240 px, with the picture at most
`ASIDE_W` (46 %) of the panel's width. The card keeps its least/most heights (`CARD`) and is centred on the picture.
`aside_room` works out the sizes. Courts, umpires and texture packs keep their picture in the strip, and so does a
character whose panel is too short for the card's least height (the old `card_room` path).
The picture drawing (store shot or select face) is pulled out into `picture()` so both places share it.

**Checked:** `HST_MENU=addmods HST_KIND=0 tools/shot.sh` at 1280×720, 1024×768 and 600×900 portrait (shots in
`context/b696/`): the picture and card sit side by side, labels and letters readable, nothing clipped.
Test `the_picture_row_comes_before_the_strip`.

## Not verified / not 1:1
- Browse (store listing) pictures weren't shot (the shots use the Installed page, no network). They go through the
  same `picture()` path as before, now in the tile.
- Front-end design is ours (PLAN.md): there is no original to match.

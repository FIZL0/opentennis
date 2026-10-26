# B666: P4's icons disappear in local player select

**Cause.** Each roster tile carries a small `1P`–`4P` tag for every player whose cursor is on it (`roster.rs`
`tags`). The tags were laid left to right along the tile's foot at a fixed size and the loop stopped at the first
one that didn't fit. A tile is about two and a half tags wide, so with three or four players on the same tile
(everyone starts on, or moves to, the same character) the last tags, 4P's first, were simply not drawn.

**Fix.** The tag row shrinks uniformly (tag height, text, padding, gap) to the tile's width when it's too wide;
a row that fits keeps its old size and place. Test: `roster::tests::every_players_tag_fits_on_a_shared_tile`.

**Checked.** `HST_SELECT=3,3,3,3 HST_MENU=chars tools/shot.sh …`: all four tags on the shared tile in every column,
small but readable (context/b666/same-after-crop.png).

**Ruled out on the way.** The menu's 3200-quad pool (overflow would drop the last-drawn column, 4P's): the busiest
four-human select measured 2585 quads live (Menu Size 50 %, icons baked), at most ~2700 over 4:1…0.4 aspects,
Menu Size 50–100 % and the confirm/venue screens, so it isn't hit. `seats.rs` needed no change.

## Not verified / not 1:1
- The user's exact situation wasn't described beyond "P4's icons disappear"; this fixes the one case found where
  4P's tags go missing. If they meant something else (e.g. icons vanishing while scrolling), it needs a repro.
- Front-end only (no original to match: our own select design).

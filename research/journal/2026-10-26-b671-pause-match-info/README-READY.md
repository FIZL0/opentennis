# B671 — Pause menu shows the match setup (concept round 1)

## What's on the branch
`concept/pause-match-info` (commit d7a3ab62, pushed). The pause screen now shows the match setup:
- each player's name, character and a 1P/COM tag, with both teams in doubles
- the rules in play: Offbeat rules, "No shot markers", handicap weights, swapped hands

It reuses the pause screen's own art (pill/box art, the score board, slot tags, word labels) and placement
(`widescreen::place`), so it scales uniformly at any aspect ratio. The code is in `crates/hst/src/play/menu/info.rs` and
hooks into `menu.rs`. It's presentation only: nothing touches the sim, RNG or match timing.

## How to try it
`git switch concept/pause-match-info`, start a match, pause. Pick a layout with `HST_PAUSE_INFO`:
- `cards` (default): a Players card on the left, a Rules card on the right.
- `board`: the names sit on the score board's plates, with the rules as chips in a column.
- `chips`: a "1P Ryan vs COM Big Chief" row under the top bar, with the rules as chips down both sides. In doubles, team A
  and team B each get a column.
- `strip` (the user's request, after the original's score strip): the board is widened to full width, using the
  original's striped plates and beige frame. Each plate carries its tag, the player name and the character name. The
  rules go in a Rules card.
- `off`: shows nothing extra (the old screen).

Shots (git-ignored) are in `context/B671/`, each style in singles and doubles at 1280×720 and 720×1080:
- `r4/` strip
- `r4-3sets/` strip, cards and chips with the taller 3-set board
- `r5/` and `r5-3sets/` chips after the column fix
- `base/` before the change

## What's next
The user picks a style, or a mix (e.g. strip plus chips for rules). Then:
- drop the other styles and the flag
- add a release-notes line
- merge into main as a normal task

## Not verified / not 1:1
- All layouts are our own design: the original's pause screen shows no names or rules. Only the strip's look (plates,
  stripes, frame) follows the original's board.
- No live interactive check in a running window. These were checked with `--shot` captures only, plus a unit test that
  keeps every layout on screen and clear of the board and dialog.
- A mod character shows its mod name only. Texture packs aren't listed. Court and umpire show only where the top bar
  already showed them; the default court's name is empty there (an existing gap).
- COM faces show as white discs in the shots, the same as in the baseline shots. That isn't from this change.
- The 3D name markers over the players still draw above the pause screen, the same as before.
- Online matches weren't tried. The names come from the same `Names`/args the match uses.

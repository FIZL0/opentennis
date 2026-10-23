# B659 — Split view: pop-ups centred, each half shows only its team's score at the top

## Cause
The umpire's calls (Fault, Double Fault, Out, Let, Net, Change Sides) are 3D models that ride the main camera
(a child of it, scaled to its field of view). Split, the main camera draws only the first half (its viewport),
so the calls showed only over the left (or top) half. The 2D pop-ups (point boards, banners, speed) already
covered the whole window.

## Change
- `split_view`: the `ON` flag (stacked only) became `split_shown()`: the split's orientation, side by side or
  stacked, whenever the picture is split.
- `popups`: split, the calls' space is unparented from the main camera (identity transform), the call models move
  to the overlay camera's layer (`split_layers`, only on a change), and the overlay camera (now order 3, above the
  second half's order 2) runs while a call plays. It draws over the whole window with its own 50° picture, as the
  intro's banners do. Not split, the calls behave as before (the layer goes back to 0).
- `panel`: split, `halves` lays out the panel twice, each time with one team at the top (team 0's on the left,
  team 1's on the right), keeps only the top half's quads and moves the second team's 448 down. `Room::place` lays
  each half out as if it were its own window (`widescreen::half_aspect` for the pinning and scale), then maps it
  into that half (`widescreen::in_half`). The touch pad's drop applies only to halves at the top of the view. The
  panel's name plates (`name_tags`) go through the same `Room::place`.
- Before this, stacked split put the near team's panel on top and the other team's at the bottom. Side by side,
  both teams' panels showed across both halves.

## Checked
`--shot` (`context/b659/run1/`, `HST_SPLIT2=on HST_CALL=k@f HST_WEATHER=0`, court 1):
- `wide_fault.png` (1280×720 singles, 1P k + 2P pad): Fault centred on the window. 1P's score is at the top left
  of the left half, 2P's at the top right of the right half.
- `tall_change.png` (450×900 stacked): Change Sides centred across the seam. 1P's score is at the top of the top
  half, 2P's at the top of the bottom half.
- `wide_doubles.png` (doubles, 1P and 3P): Double Fault centred. A Team's pills and banner are on the left half,
  B Team's on the right.
- `nosplit_fault.png`: no split, panel and Fault as before.

Tests: `panel::tests::each_half_its_own_team`, `widescreen` `halves`; panel/split_view/name_tags/popups suites pass.

## Not verified / not 1:1
- Remaster-only presentation (the original has no split), judged by eye.
- On phones the halves are taken from the HUD's view (inside the safe area), but the 3D halves split the full scene
  rect. With safe-area insets, the panel's half seam can sit a few px off the picture's seam. Not tried on a phone
  or with the touch pad (split needs two local humans).
- The calls through the overlay camera use its own 50° fov fitted to the whole window, so they come out the size
  they'd be on one full picture, not each half's.

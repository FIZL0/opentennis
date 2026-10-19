# P63 — Steam Deck preset at High, supersampling

- `Graphics::STEAM_DECK` is now Default's look (Shadows High, 100 %, 4x MSAA) with vsync on and the speed-ups
  (Fewer Shadows, Skip Off-Screen) on, i.e. High's picture plus the Deck's optimisations. A settings.txt that holds the
  old Deck preset exactly (Low shadows, else the same) is moved up on read (`Graphics::upgrade`, called from the
  menu's `Settings::parse`); window/Show FPS/Auto are kept.
- `SCALES` gains 125/150/200 %; `render_scale` is read up to 200. `hud_gamma::scene_scale`: up to 100 % as before
  (min with the density cap); above it the capped size times the scale, never past the GPU's
  `max_texture_dimension_2d` on the longer side. The composite's linear sampler filters it down (2x = exact 2×2 box).
  `cheaper` (Auto's step-down) steps 200 → 150 → … like any other scale; Auto never picks above 100.
- Options' Resolution help text and the Deck preset's hint/description updated.
- Check: `--play --msaa off --scale 100` vs `200` shots (context/p63/cmp.png): edges clearly smoother at 200.

## Not verified / not 1:1
- Not run on a real Steam Deck (no device here); the preset change follows the user's report that the Deck holds High.
- 125/150 % downscale through the bilinear sampler is not a proper filter (some residual aliasing, no extra blur);
  remaster-only rendering option, looks right by eye.
- No original to compare against: both are remaster-only settings.

# B636: blue players in the split view's bottom half

## Cause
Not a missing draw. Bevy copies each camera's picture onto its target at the end (the upscaling blit), and every
camera after the first on the same target is blended in by its alpha unless it says otherwise. The split's second
camera (`split_view.rs`, order 2) shares the scene image with the main camera, so its half went on with
ALPHA_BLENDING. The court, props, umpire and NPCs write alpha 1; the players' draws write their texels' alpha (under
1), so their pixels came out partly whatever the scene image held under the bottom half (a dark blue there). That's
why it followed the second camera entity, ignored lighting/fog changes, and why the B633 "texture × 0.5" test seemed
not to reach it (the colour was drawn, then mostly replaced by the blend).

Found by bisecting gs.wgsl's output in the left/right halves of the window (`context/b636/run1/`): all-red output
reached the bottom players (so their draw runs), texture-only and vertex-colour-only outputs at alpha 1 were right,
skipping the shadow did nothing, forcing alpha 1 alone fixed them.

## Fix
The split camera's `output_mode` is `Write { blend_state: REPLACE, clear_color: None }`: its viewport (the bottom
half) is copied over as drawn. The blit is limited to the camera's viewport, so the top half is untouched. Main
camera, landscape and the sim unchanged (the second camera exists only while split).

## Checked
450×900 doubles, court 1, `HST_WEATHER=0`, `--shot-at 16` (`context/b636/run2/split.png`): both teams' players in
their own colours in the bottom half, top half as before.

## Not verified / not 1:1
- Remaster-only presentation (no original counterpart); judged by eye on one court/moment.
- No release note: the split view (B633) hasn't been released yet, so no player saw the blue.

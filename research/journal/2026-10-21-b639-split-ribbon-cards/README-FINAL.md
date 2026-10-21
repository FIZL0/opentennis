# B639: split view, the ball's ribbon, bounce puffs and court cards face both halves

B634 follow-up: the ball's flight ribbon, the bounce puffs/dust and a court mod's card people (`billboard: true` in
`npcs.json`) were still turned to the main camera only.

## What changed
- `effects.rs`: the ribbon mesh and the bounce puffs and dust meshes are `twin_view::Billboard::BUILT`;
  `draw_flight` / `draw_bounce` build their quads per camera (`ribbon(…)`, a `facing` closure) and fill the copy's
  mesh for the second eye (with its own lens for the ribbon's width). The glow disc faces the flight, not the
  camera, and the bounce marks lie on the ground: unchanged.
- Cards can't go through `Billboard`: a card is a whole glTF scene (a person root turned to the eye, node tree,
  clips, maybe a skin), and a lone mesh copy can neither be turned about the person nor follow a skin. So
  `court_people::spawn` loads each card's scene a second time (`Twin`: root, clips, playing, head), hidden while there's
  no split. `show` drives both: the copy turned to the second eye, with its own clip pick (the back clip by
  that camera) on its own nodes, and a line judge's head after the ball (`act`, shared). `halves` (PostStartup, after
  spawn) marks the card's own meshes `twin_view::TopHalf` (new: on `TOP` while a second view is on, back off after,
  `top_halves`) and the copy's meshes `RenderLayers` `BOTTOM`. `show` now runs after `split_view::split` so the eye
  is this frame's.
- Test: `a_court_mod_is_seen_and_heard_and_never_moves_the_match` (its stand-ins are cards) checks each card has a
  copy, the same mesh count, own meshes `TopHalf`, copy meshes on `BOTTOM`, copy hidden with no split.

## Checked
`tools/shot.sh`, 450×900, court 1, all CPU, `HST_WEATHER=0`, frames at 14–17 s (`context/b639/run1/`): the
ribbon shows in both halves with width from each camera (`c.png`: bottom half sees it head-on, still a band, not an
edge), bounce dust puffs show in both halves.

## Not verified / not 1:1
- Court and scenery are missing from `--play` shots in this slot (B640), so the shots are on a blank background.
- No before/after pair for the ribbon: from the far end the old ribbon (a strip facing the top camera) looks
  similar except near edge-on angles; judged by code and the after shot.
- Card people weren't seen on screen: no installed court mod has `billboard` people (the Get a Grip courts have
  none); checked by the court mod match test's structure check only.
- A card copy costs a second load of its scene (meshes, materials) for the whole match, split or not.
- Rendering only (presentation): no PINE proof needed; the match hashes in the court mod test stay the disc court's.

# B671 — Pause screen: score panel with player and character names (panel B, on main)

The user picked board B "Original footprint" (Compact.dc.html) from round 2 and then moved B671 off the concept branch
and onto main. Round 1's layouts (cards/board/chips/strip) and `HST_PAUSE_INFO` stay on `concept/pause-match-info` and
are not used. The rules tab is B671a.

## What changed
- `crates/hst/src/play/menu/plate.rs` (new) builds two images per player, while paused, and rebuilds them when they
  change (e.g. a controller is handed over):
  - The name goes where the 1P..4P/COM slot tag was. It is the player's picked name, or the seat tag ("1P") if they
    have none. A CPU always shows "COM".
  - The name is set in M PLUS Rounded ExtraBold, white, with a 2-px outline in the plate's dark team colour (pink
    #8a1f52, blue #14507e, orange #7a3f0a, green #3b6213, from the board).
  - The name is 22 disc px tall, in the tag's room between the face and the centre. The left team's name sits
    against the centre and the right team's starts from it. A long name shrinks (aspect kept) to fit.
  - The character name (a mod uses its own name) sits on a navy (#16213d) pill, 11 px tall, centred under the face
    and overlapping its foot. It is kept on its plate and shrinks if it's too long.
- `menu.rs`: the `Tex::Name`/`Tex::Chara` quads replace `Tex::Slot` once the words exist. The plates, frame, centre,
  faces and digits are unchanged (the digits were already our own font, `menu_art`). Everything uses the board's own
  `Centre` placement, so it scales uniformly at any aspect ratio.
- `hst-play` `name_tags.rs`: `panel_words`/`panel_pill` render the images (at `menu_art::K`×).
- Test `plates_carry_names`: the tag is gone, names and pills stay inside their plates (short and very long, singles
  and doubles), they're aligned, and CPU/name/tag word choice is right.
- Release-notes line added.

Shots: `context/B671/b2/` (singles/doubles × 1280×720 / 720×1080; doubles with "Alexandrina" to show shrinking).

## Not verified / not 1:1
- This is our own design (the original's pause shows only 1P/COM tags), checked by `--shot` against the mockup, not
  against the original.
- A very long name (e.g. "Alexandrina") gets small: the tag's room is only about 37 disc px wide. Per the brief it
  shrinks rather than wraps or overflows.
- A name with an accent above the caps may clip at the image's top edge. Not tried.
- The 3D seat markers over the players still draw above the pause screen, as before; this change doesn't affect them.
- Online matches weren't tried. The names come from the same `Names`/`Pads` the match's own plates use.

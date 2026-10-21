# B632: main menu Settings preview overflows in portrait

## What was wrong
At Aspect Auto and Menu Size Auto, any window 4:3 or narrower (a phone held upright, 3:4, 9:19.5) gives the main
menu its 4:3 layout: the preview panel is 344 units wide, not 540. The Settings picture kept its page pills at a
fixed 150 units and only fit one across, so its five (six with touch: Mobile) pages stacked in one column down past
the picture, over the row's name and the description. The empty Mods picture's one-line "No mods installed yet…"
also ran past the picture's right edge.

## Fix (`crates/hst/src/play/main_menu/home.rs`)
- `settings_pills`: as many across as it takes to keep the rows clear of the row's name over the picture's foot
  (three rows at most in the 170-high picture), pills narrowed to fit; a wide panel still gets three across at 150.
- The empty Mods line is wrapped (`para`) to the picture's width.
- Test `settings_pills_clear_the_name`: pills inside the picture and above the name for the panel widths at 9:19.5,
  3:4, 4:3, 16:9, 21:9, five and six pages.

I kept the preview beside the list rather than stacking it underneath: once the pills fit, nothing in any row's
preview overlaps at 9:19.5 or 3:4. How big the menu draws in a tall window is the business of the sibling portrait
tasks (B630/B631).

## Checked (`--shot`, Menu Size Auto from a scratch settings copy; `context/b632/run2..4`)
- Settings at 400×866 (9:19.5) and 600×800 (3:4), with and without touch: two across, three rows, clear of the name.
  At 16:9 it's unchanged (three across, two rows).
- At 9:19.5: Local, Online, Mods (empty, wrapped), Extras, Rematch and the New version card. Nothing overlaps.
  The Update picture was half hidden by the update prompt in its shot. Its tiles size to the panel width.

## Not verified / not 1:1
- Not checked on a real iPhone: desktop `--shot`s with `HST_TOUCH=1` at the phone's aspect stand in for it.
- I didn't take a shot of the old build to confirm the overlap. The cause follows from the layout maths: one pill
  across at panel width 344.
- The Mods picture with installed mods wasn't shot in portrait (no mods in the scratch folder). Its tiles already
  scale to the panel width.
- Menus aren't 1:1 with the original (PLAN.md), so there's nothing to match there.

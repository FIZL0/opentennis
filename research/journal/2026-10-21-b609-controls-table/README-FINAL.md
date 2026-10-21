# B609 — Controls as one scrolling table

## What changed
- `options.rs` `controls_layout`: no Controller / Keyboard & Mouse tabs and no side panel. One panel takes the page's
  whole width (a portrait view's whole height). Its blue strip carries the column headings "Action · Controller ·
  Keyboard & mouse", and under it there's a row per action: name, a dark Controller cell (pad icons plus the fixed
  stick glyph) and a white Keyboard & mouse cell (key caps). The chosen cell gets a white halo and a thicker outline;
  the one waiting for input turns blue ("Press a button..." / "Press a key...").
- Rows have a least pitch of 52 mockup px (Settings rows are 46). As many as fit whole are shown, spread to fill the
  room up to 65 px. When they don't all fit, the table scrolls with the cursor (`Menu::controls_top`, which moves
  only when the cursor would leave the view) and a scroll bar shows. The pad's ↑↓, the wheel (`Act::Wheel` → ↑↓) and
  a finger's drag (it sends ↑↓ presses) all move the cursor, so they all scroll the table.
- ←→ (or a horizontal drag) choose the row's cell; `keys_tab` now means "the Keyboard & mouse column is chosen", so
  the rebind system is unchanged. L2 / R2 no longer switch tabs. Legend: Choose / Rebind / Page / Back.
- Reset to defaults is a square button (rounded corners, round-arrow mark) at the right end of the Settings bar,
  where the page tabs end. In a portrait view it sits at the end of the tabs' row. In the cursor order it comes
  above the first row: ↑ from row 0, ↓ from the last row. It has its own legend ("Reset to defaults") and footnote.
- Pointer: every cell is a hit of its own (`options::controls_cell`: row = 2·action + column, Reset = 2·n), chosen
  through `Menu::controls_choose`. A tap on the action's name rebinds the column already chosen.
- `chrome` is split into `settings_bar` (title band + page tabs, with room left at the right for the Reset square)
  and `footnote`, which the table shares.
- The pause menu's Settings draws the same `Menu` layout, so it gets all of this unchanged.
- Debug screens for `--shot`: `HST_MENU=controls-last` (scrolled to the last row), `controls-reset` (Reset chosen).

## Checked
- Tests: `options::tests::controls_table` covers cells, Reset order, scroll following, wheel, cell taps and the Reset
  tap. `pages_fit` checks the table rows on view at 7 aspects (≥3 rows, ≥40 px high, over the footnote) and the
  quad pool. The pointer test's Controls taps were updated. All 254 `main_menu` tests pass.
- Shots: 16:9, scrolled to the last row, Reset chosen, 1600×600 with a rebind waiting, and 540×960 portrait (all
  13 rows fit, no bar), in `context/b609/run1`, `run2`.

## Not verified / not 1:1
- Front-end, ours to design (PLAN): no original to compare with.
- Not tried on a real phone or with a real touch drag. Drag scrolling goes through the same ↑↓ path as other
  lists and is covered by the unit tests only.
- The in-match pause Settings wasn't shot separately: it draws the same layout code.

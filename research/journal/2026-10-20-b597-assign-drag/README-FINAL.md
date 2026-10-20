# B597: drag a card on the Controllers screen to swap seats

The same screen serves the main menu and the pause menu's Controllers pill, so one change covers both.

- `pointer.rs`: when the left button goes down on a card that can move (`Menu::draggable`), the click waits.
  Moving more than 6 px (448-high units) turns it into a drag: `Act::Carry(seat, grab, at)` is sent each time the
  mouse moves, then `Act::Land(seat, at)` on release. A press that never moves still sends the old click on release.
  While the button is held, hovering doesn't move the focus. `to_menu` turns a view point back into the menus'
  centred 640×448 units. Touch is unchanged: a finger drag still steps the cursor.
- `assign_drag.rs`: `drag_swaps(p, q)` uses the same rules as the ←→ carry, but for any two seats: 1P is never
  left empty, over a match the computer's seats stay fixed, and online only this machine's seats can move, with
  nothing under the Keep / Undo popup. Landing on a valid seat swaps the seats (and the computer players before a
  match), sets `assign_lead` as the held swap does (B437), and moves the focus to the card. Drawing: the lifted card
  is drawn last, offset by the drag and given a soft shadow; its seat shows a dashed outline in the seat's colour,
  and the drop target gets a gold ring. Hit rects stay where they were.
- `HST_MENU=assign-drag` is a shot fixture: doubles, 1P's card dragged most of the way onto 2P's.
  Shots of it at 1280×720 and 600×900 are in `context/b597/` and look right; the tall window shows two rows of two.
- Tests: `assign_drag::tests` (a swap, a drop off every card or back on its own seat, the rules, and the lifted
  card drawn offset at five aspect ratios within the pause pool).

## Not verified / not 1:1
- This is our own front-end, with nothing to match in the original (PLAN: menus aren't 1:1).
- Not tried by hand with a real mouse, because this is an unattended run. The pointer flow (press, then carry,
  then land or click) is covered only by reading the code and by `Menu`-level tests; there is no test that drives
  the Bevy pointer system itself.

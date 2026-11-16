# M164 — Swap ✕ and ○

A Controls setting that trades every controller's ✕ (South / A) and ○ (East / B), off by default.

## What it does
- **Storage**: global, one more line in `controls.txt` (`swap_cross_circle = on|off`), beside the bindings, since
  those are shared by every player (no per-pad controls settings exist). Reset to defaults turns it off too.
- **Input** (`hst-play/src/play/controls.rs`): with it on, right after Bevy's input systems (PreUpdate, before
  `mode::hush`) each real pad's South and East states are traded (held / just pressed / just released / analog
  value); just before Bevy's input systems next frame they are traded back, so the frame's raw events land on the
  physical buttons. Everything downstream (the menus' hard-coded South/East OK/Back, the pause/result screens, the
  recover prompt, the intro skip, the bindings, the shots) sees the swapped *logical* button. The match, its
  recordings and netplay only ever see logical buttons, so nothing in the sim or the wire changed.
- **Not swapped**: the on-screen touch pad (iPhone/Android/browser), whose buttons are drawn as ✕/○ and labelled with
  what they do; and the keyboard (its keys are rebindable on the same page).
- **Art** (`hst-play/src/play/legend.rs`): `face()` maps the bottom/right face art when the swap is on, in the legend's
  Ok/Back glyphs, the per-button icons (Controls table, "Press ✕ to join", Leave pills) and the join hint art; the
  baked glyph images are rebuilt when the setting changes, and legend chips are keyed by it.
- **UI** (`hst/src/play/main_menu/options.rs`, `main_menu.rs`, `main_menu/legend.rs`): a "Swap ✕ ○ On/Off" pill
  left of Reset on the Controls strip; ← from Reset reaches it, ✕ or a tap flips it and saves. Legend: Choose / Swap.

## Checked
- Unit tests: `swap_line_round_trips`, `swap_trades_cross_and_circle` (through Bevy's real gamepad processing, held
  across frames, release, touch pad untouched, turning off), `controls_table` extended for the pill (keys + tap).
- `--shot`s (context/m164/): swap on with PS art (Topspin ○, Slice ✕, legend ○ Rebind / ✕ Back), off with Xbox art,
  the pill selected, and flipped live with Enter (art re-baked, controls.txt written).

## Not verified / not 1:1
- Not tried with a physical controller in a match (no pad on the runner); covered by the gamepad-processing test.
- The touch pad and the keyboard don't swap (deliberate, see above).
- Minna no Tennis's own layout isn't reproduced beyond the ✕/○ swap (△/□ unchanged), as the task asks.

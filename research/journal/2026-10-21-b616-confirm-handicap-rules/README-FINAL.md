# B616: Confirm screen: Handicap and Offbeat Rules made visible (concept A)

## What changed

- **Handicap stepper on each local card** (`handicap.rs` `stepper`, `Foot`): a HANDICAP caption, a pill
  `‹ weights · "1 weight · runs 20% slower" ›` as wide as its words, the △ / keyboard key glyph after it, Ready! at
  the end of the line. The effect is `100 − hst_sim::player::handicap_size(h)` (`effect`, `words`), never a written
  number. Narrower cards step down: one line → stepper line + help/Ready line (`Foot::Two`) → icons + `xN` with Ready
  full width (`Foot::Small`). The card's footer height follows (`roster::columns_stepped`). Taps: left half of
  the pill steps back, right half forward.
- **Online lobby pill** (`handicap::pill`) shows the same words ("No handicap", "1 weight · runs 20% slower") when they
  fit, else the short label.
- **Offbeat Rules cell** in the local strip (`confirm.rs` `rules_cell`): "None on" / "N on", gold chips of the rules
  on (Bounce, Slow-mo, Turbo, Ghost, Tiny/Big, Fast/Slow, Sweet spot; "+N" past the room), the Select/C glyph only on
  the confirm screen (on the select, C is Hand). ✕ on it (new confirm row `Menu::rules_row`) or a tap opens the
  existing rules box; Select/C still opens it. Hidden online (the lobby's cells stay Court/Umpire/Sets/Games/Mods/
  Markers).
- **No squish** (`frame::foot`): the foot no longer scales down. It takes the first layout whose cells keep their
  least widths at the current Menu Size: one row (cells + Rules + Start) → the strip on its own row with Rules +
  Start under it (online: Start full width) → 2×2: Court | Umpire, Match | Rules, Start full width. In the 2×2
  layout Sets, Games and Markers share a **Match** cell ("1 set, 4 games"); ✕ / a tap opens a small panel
  (`match_panel.rs`) with ‹ › steppers, ↑↓ choose, ←→ change, ✕/○ close. Online 2×2 is three rows of two cells.
- **Cards keep their least height**: a card needs its top, rail, footer and one whole tile row. On a short window a
  big Menu Size can't give that under a two- or three-row foot, so `fit_scale` lowers the Menu Size for the select
  and confirm screens until it does (`confirm::cards_fit`, one to four players), never below the size that fills
  the screen. This is the same rule Settings pages already follow (B608).
- Release note line added.

## Shots (context/b616/)

`run3/<screen>-<aspect>-<menu size>.png` for `confirm-handicap` and `confirm-offbeat` at 16:9 (1280×720), 21:9
(1680×720), 4:3 (960×720), 16:10 (1152×720), 9:16 portrait (600×1066) and a phone-sized landscape window (844×390),
each at Menu Size Auto, 50% and 150%; `s3-*.png` are contact sheets (Auto | 50 | 150). `sheet-lobby.png`: the online
lobby at 16:9 and 4:3, the select at 4:3 / 150%. No clipped words found in the strip, the steppers or the panel;
16:9 Auto lays out as one row; 4:3 and portrait drop to the second and third layouts.

## Tests

- `match_panel::the_match_panel_steps_sets_games_and_markers` (new).
- `confirm::a_short_window_lowers_the_confirm_screens_menu_size_until_the_cards_fit` (new).
- `online::arrows_cross_the_lobby_cards`: Start is under the strip at 4:3 now, inline at 16:9 (both checked).
- confirm / handicap / offbeat / frame / roster / lobby / legend filters pass.

## Not verified / not 1:1

- **The concept canvas wasn't accessible**, so the look follows the task text, not the mockup: sizes, colours,
  chip names and the Match panel's look are mine.
- **Least widths are fixed board px per cell** (`FOOT_MIN`, `FOOT_MIN_LOCAL`), not measured words (the geometry is
  computed without the font). A very long court, umpire or mod name still ellipsizes inside its cell (e.g. the
  lobby's Mods "Allo…" at 4:3), as before.
- **150% doesn't hold on short windows**: at 720 lines and below, 150% can't fit a card's least height over the foot,
  so the select/confirm screens come down to the largest size that does (Settings pages do the same). Online
  confirm/lobby are capped by the local card check (an estimate for them).
- **Phone size** is a 844×390 desktop window, not a touch device (touch Auto's size is B608's, not checked here).
- The Match panel is local only; online, the 2×2 fallback is three rows of two cells with each stepper kept.
- Doubles (four cards) wasn't shot; `cards_fit` checks one to four players and the steppers step down to `Small`.

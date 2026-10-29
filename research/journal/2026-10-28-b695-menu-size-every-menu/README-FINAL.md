# B695 — Menu Size above 100% grows every menu

The title, Main, Online, Lobbies, Join by Code / relay URL keyboard and Extras were capped by `fit_scale` at the
size that fills the view ("whole"), so 110–150% looked like 100% on a landscape window. They now re-lay out on the
view (`Grid`) at any Menu Size, and `fit_scale` no longer caps them.

## Why a plain "fit the block" doesn't work
The title tab, info bar and legend grow with Menu Size too, so the room left for the content (in 640×448 units)
shrinks faster than 1/k: a fixed block scaled to fit gets *smaller* on the window as k grows. Every screen therefore
reflows: gaps close, pictures give way, lists scroll, side panels move or hide, and only as a last resort a block
shrinks.

## What each screen does on a short / narrow view
- **Main** (`home.rs`): rows close their gaps down to 42 (pills keep 38 high and their 24 words), then scroll over
  Quit with a scroll bar (`home::scrolled`); Quit sits at the view's foot. The preview panel: picture shrinks first
  (to 90), then the facts go, then the words' last lines (`panel_fit`), cut words end in "..." (`para`). Too narrow for
  the panel (under 230 units beside the list): the list centres and the panel hides. The pads/disc line in the corner
  shrinks to keep clear of the title tab.
- **Extras**: same panel rules; too narrow to sit beside the row, the Donate panel goes under it.
- **Online**: big pills keep their height, gaps close to 4, then scroll; the card follows the panel rules or hides.
- **Lobbies**: rows that fit the view (1–6), list narrows (min 480 / view − 140), court column dropped under 400 wide
  (and the other-version row drops its kind); the searching / none / failed panel moves/shrinks between the rows and
  the note.
- **Code / relay URL** (`code_fit`): block moves up under the tab, keys narrow to the view (58 → 48 across, buttons two
  keys wide), keys flatten (to 26–38 high, letters stay 24), and only then the block shrinks. 8 columns kept, so
  pad/keyboard navigation is unchanged.
- **Title** (`splash::fit`): prompt lifts clear of the legend, logo shrinks (to 120 high at least) into what's left.

## Proof
`main_menu::tests::every_menu_grows_with_menu_size`: the six screens plus the relay-URL keyboard at 16:9, 21:9, 4:3,
19.5:9 and 9:19.5, Menu Size 100–150% in 10% steps: every Centre-anchored quad stays inside the view, and the words on
the buttons (the title: its biggest words) grow on the window by ≥2% each step. `menu_size_fits_the_view` now asserts
the six take 1.5. Shots at 100/150% (16:9 1280×720 and 4:3 960×720) in `context/b695/` looked right by eye after two
fixes (keyboard entry overlapping the first key row; the other-version lobby row squeezed at 4:3).

## Not verified / not 1:1
- Menus are ours (PLAN: front-end doesn't have to match the original), so nothing compared against the PS2 game.
- No phone screenshot: the phone aspects are covered by the layout test only, not by eye (no device/emulator used).
- At 150% on a narrow view some content gives way rather than grows: Main's and Online's preview card hides at 4:3,
  the Main preview's picture and the panels' text lines shrink/cut ("..."), the corner pads/disc/version print stays
  the same size on the window once it reaches the title tab.
- The touch-typing variant of the Code screen (no key grid) and the stale-version / update / quit popups were not
  shot; they sit in the screen's middle (popups ≤ 160..330) and fit the 150% views by arithmetic.
- The relay-URL keyboard at 16:9/4:3 150% grows only ~3% over 140% (its 6 rows of keys flatten to 26 high).

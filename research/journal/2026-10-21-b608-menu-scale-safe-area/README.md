# B608 — Menu Size above 100 %, phone Auto size, Safe Area — FINAL

## Menu Size above 100 %

Baseline shots at 150 % in a 16:9 window and a sideways phone (852×393): the screens drawn on the fixed 640×448
screen (title, main menu, Online, lobby list, code/name keyboard, Extras) grew past the view and were cut off on every
side. The screens that re-lay out on the view (`Grid`: select, lobby, mods, mode, assign, confirm, settings pages)
already reflowed; a Settings page at a big size on a short view squeezed its rows instead.

Fix (`main_menu.rs` `fit_scale`, `options.rs` `rows_fit`): the Menu Size asked for is capped per screen, in
`apply_menu_scale_on` (menus and the pause menu's settings):
- fixed-screen menus grow until the 640×448 screen fills the view and no further (nothing past its edges);
- a Settings page grows until its rows would drop below their full pitch, in 5 % steps;
- the rest take any size.

Shots (in `context/b608/b`, `c`, `p`, `s`): at 150 % every menu fits in 16:9 and on the phone, nothing clipped.

## Auto on a phone

`Settings::menu_scale_on`: on a touch screen Auto makes the 448-unit screen at least 440 logical px tall
(≈ 2.75 in of glass at the phones' ~160 pt/in), clamped to 100–130 %: a phone held sideways (393 pt) gets ≈ 112 %, a
small one (320 pt) 130 %, a tablet 100 %. Logical px are the OS's density-independent points, so this follows the
physical size, not the pixel count. Desktop Auto is unchanged (100 %, 80 % above 1440 px).

## Safe Area (user clarification 2026-10-21)

`widescreen.rs`: the picture's rect (`scene_view`) is now the whole window (or the fixed ratio's rect centred in it),
never shrunk by the safe area; the HUD's and menus' rect (`view`) is that rect less the insets. Each edge's inset is
the larger of the device's (`SAFE`: iOS safeAreaInsets, Android cutout, the page's env(safe-area-inset-*)) and the
user's margin. `hud_gamma::follow_window` sizes the scene and HUD images to the picture's rect and gives the HUD's
camera a viewport over the view's part of its image (the whole image is still cleared, so nothing is drawn outside).
Everything placed in the view (HUD pinning, legend, pointer, the browser's mod picker) follows unchanged.

New Settings → Display → Safe Area: Auto (device insets), None, 1 %, 2 %, 3 %, 4 %, 5 %, 7 %, 10 % (per edge, the
device's still kept where bigger), saved as `safe_area =` in settings.txt. Stretch (B421) is no longer offered: Auto
now does what it did for the picture, and a saved Stretch reads as Auto.

Checked with `HST_SAFE=59,0,59,21` (iPhone 15 Pro sideways) at 852×393: the menus and the match HUD sit inside
59 px left/right and 21 px bottom; the 3D view spans the whole window (player positions identical to a run with no
insets).

## Not verified / not 1:1

- No real iPhone, Android phone or phone browser: the insets were faked with `HST_SAFE`; the reading of the device
  insets is the existing M106/B590 code, unchanged. Both landscape directions only differ in which side has the
  inset, which the fake covers by having both.
- The in-match shots show no court: the `--play` shot run in this setup drew no court with or without the change (the
  clear colour filled the window edge to edge, players and HUD were drawn); the court reaching under the island is
  inferred from the scene image and viewport now being the whole window, not seen.
- In menus the strips outside the safe area show the picture behind the menus (the title backdrop is a UI image, so it
  stops at the safe area and the window's clear colour shows there), not the menu's own wall.
- Browser portrait keeps B314: the canvas still starts below the top safe area (status bar), so in portrait the picture
  doesn't reach under it.
- Phone Auto size is a logical-px rule (OS points ≈ physical size); no per-device DPI table.
- Controls' binding icons stay small at big sizes; the Controls page becomes one scrolling table in B609.
- Menus aren't meant to be 1:1 with the original (PLAN.md); nothing here is gameplay.

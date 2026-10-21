# B620 — Steam Deck 1280x800 layout

Report: at the Deck's 16:10 resolution text and menus draw wrong. I could not reproduce it; nothing in our own
renders at 16:10 is misplaced, clipped or stretched. Needs the user's screenshot and how the game was run.

## What was checked (shots in `context/b620/run*/`)
- `--shot` at 1280x800 (also 1440x900, 1680x1050), scale factor 1: main, settings (display, graphics + Preset's
  dropdown, sound, controls), game selection, chars, confirm, roster4, mods, browse, umpire-mod select, online
  (Relay), lobby, lobby with a bot, the renderer popup, the controller takeover prompt, the match HUD, its start
  (score panels sliding in) and the pause screen. All uniform-scaled, nothing clipped or stretched.
- High X11 DPI on the Deck: X can report the 7" panel's ~216 dpi, a 2.25 scale factor (569x356 logical). Added
  `HST_WINDOW="W H SCALE"` (`main.rs`; the third number is new) to fake it: `HST_SIZE='1280 800'
  HST_WINDOW='1280 800 2.25' tools/shot.sh …`. Layout is identical at 1.25, 1.5 and 2.25. At 2.25 the HUD image is
  capped at 2 px per logical px (`hud_gamma::MAX_DENSITY`, a phone setting), so small text is slightly soft. That's
  by design, not this bug.
- Game Mode: ran the release build fullscreen (no `HST_BG`, Display = Fullscreen, `SteamDeck=1`) inside
  `gamescope --backend headless -W 1280 -H 800 -w 1280 -h 800`. winit guesses scale 1 and the window comes up
  1280x800. The main menu is correct (the shot was `HST_KEYS` `shot=`).
- Aspect Ratio = 16:10 on a 16:9 window: the select is drawn in the centred 16:10 rect with bars, correct.
- At 16:10 the select's cards are narrower in 640-units than at 16:9, so the 1P card drops its `[N] Name` pill
  (its 48-unit name room rule) and the section tab reads "Hot Shots Golf »". At 4:3 it does the same, so this is the
  designed fallback, not a fault (Name stays in the key legend).

## Changes
- `HST_WINDOW` takes an optional scale factor (test tooling).
- Layout tests now sweep 16:10 (1.6) too: confirm, frame, court picker, splash, roster (×2), mods screen,
  assign, assign drag, the legend strip. `fixed_ratio_views` gains the Deck's 1280x800 at Auto/16:10/16:9. All pass.

## Not verified / not 1:1
- Not run on a real Steam Deck (no device here). The fault the user saw isn't reproduced by `--shot` at 16:10, a
  faked 1.25–2.25 scale factor or a headless gamescope at 1280x800. Unknown: Game Mode or Desktop mode, the
  Linux or Windows (Proton) build, Display/Aspect/Menu Size settings, which screen.
- Not checked: the result screen (needs a whole match) and the touch pad overlay (the Deck's touchscreen turns it
  on after a touch).

## Unblocks
A Deck screenshot of the wrong screen, plus Game/Desktop mode, the build, and the Display settings. Or the log
line `Guessed window scale factor` and the window size from a Deck run.

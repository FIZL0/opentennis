# M137 — refresh the UI screenshots

## What changed
Retaken with the current build (v1.2.5, 0ccb3fb8), same file names so README and the site (`tools/web.sh` copies
`docs/screenshots/*.jpg` to `web/shots/`) pick them up unchanged. 1920×1080, JPEG q85, metadata stripped.

- Menus: `menu-main`, `menu-chars` (four players, one column each), `menu-courts` (mod courts), `menu-umpires` (mod
  umpires), `menu-lobby` (online lobby with a computer player), `menu-browsemods` (the real store listing, with
  thumbnails).
- New: `menu-title.jpg`, the title/splash screen (Press Enter over the blurred court). README's table now opens the
  menu rows with it; its last row pairs the store with `mod-amusement.jpg` (already in the folder, unused until now).
- Match with the HUD on (M98's were HUD-off): `rally-grass` (doubles, grass: timing balloon, power-shot trail, mph),
  `serve` (doubles, grass: score panel, team names, COM levels, sets/games), `rally-autumn` (singles, autumn: power
  shot, balloon, mph).

## How
All files in `context/m137/`. `disc/` is a copy of the 2026-10-16 menu-shot folder (symlinks to the main checkout's
disc, mods, store cache; its own `settings.txt`: upscaled textures + hst-hd pack, high shadows, 4× AA, menu size Auto)
so the slot's low-quality `settings.txt` (menu size 50, render scale 67) isn't used.
- `menu.sh OUT SCREEN AT`: one `HST_MENU` screen through `tools/shot.sh` at `HST_SIZE="1200 675"` (1920×1080).
  Screens: `press`, `main`, `roster4`, `courts-mod`, `umpires-mod`, `lobby-bot`; the store is `main-mods` + Enter
  (`HST_KEYS`), shot at 45 s so the listing and thumbnails are in. The character select needs ~30 s for every face
  to be cut (earlier shots show empty cells).
- `burst.sh DIR FROM TO STEP args`: one CPU-vs-CPU `--play` match, a frame a second; `r1` grass doubles (`--stage 5
  --court 5 --chars 0,1`), `r2` autumn singles (`--stage 11 --court 11 --chars 2,3 --singles`). Picked by eye from
  the contact sheets.

## Not verified / not 1:1
- Not compared to the original: these are our own menus and renderer (front end/rendering, no proof needed).
- Not retaken: `hero`, `cast`, `celebrate-*`, `closeup-*`, `mod-castle`, `mod-character`, `mod-amusement`, `phone`.
  They are HUD-off cutaway/celebration close-ups with no menu or HUD in them, so the task (menus and match HUD)
  doesn't change them.
- The match shots are CPU vs CPU, so the HUD shows COM markers/levels rather than 1P/2P.
- `menu-lobby` is the `lobby-bot` sample screen (made-up peers "Sunny Day"/"ACE-42"), as before; no real online
  session was opened.
- The main menu's "New in the store" column doesn't show at the Auto menu size in 1920×1080 (it needs a wider
  window); left as the game draws it.
- The site was not opened in a browser; only the README table changed, and every path in it exists.

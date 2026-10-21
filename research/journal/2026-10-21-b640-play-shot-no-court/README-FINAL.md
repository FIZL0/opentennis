# B640: court and sky missing in `--play` shots

## Cause
Not a regression in the renderer. `--court N` only picks the match's court (rules, weather, NPCs, umpire); the
court's models, sky and scenery are drawn by `--stage N` (`load` → `load_court` in `crates/hst/src/main.rs`). B634's
shots ran `--play --court 1` with no `--stage`, so they played court 1 over the bare clear colour. B633's shots that
looked right must have passed `--stage 1` too (the menu and netplay always pass both).

How it was found: every build failed the same way, B633's own tip and yesterday's main (ef26abad) included, with a
fresh HST_DATA, HST_BG unset and Mesa's shader cache off, so no bisect point existed; `HST_SCENE_CENSUS` showed no
court meshes at all, and `load` only builds them under `args.stage`.

## Fix
`parse` in `main.rs`: a `--play` run with `--court N` and no `--stage` now draws court N (`stage = Some(N)`); an
explicit `--stage` still wins, court mods keep defaulting as before. Test `court_flag_draws_its_court`.

## Checked
`tools/shot.sh` `--play --court 1 --shot-at 16`, HST_WEATHER=0, clean HST_DATA: 1280×720 and 450×900 both show the
court, sky, scenery and umpire chair (`context/B640/run3/both.png`; before: `context/B640/run1/a.png`).

## Not verified / not 1:1
- none (command-line flag fix; no gameplay or rendering change).

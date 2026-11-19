# M198: match saves in the local menus, closing the gaps

M194 (pause Save and Quit) and M195 (Continue card and screen) built most of M198's line already. What was still
missing:

- **Save and Quit wrote no setup flags**, so Continue turned away every save it made ("Saved without its setup"; M194's
  journal noted this). Now `match_mode` (`crates/hst/src/main.rs`) puts the match's start flags in the world as
  `save_game::StartFlags` and `menu.rs`'s `save_match` saves them as `Setup::flags`. A command-line `--play` start
  keeps its own flags too (`match_flags`: everything after the disc, minus `--shot`/`--shot-at`), so its saves resume
  from the menu the same way. Rematch reuses the same mode, so it keeps them.
- **Browser builds**: `Saves` reads and writes through a small `disk` module: on desktop the files as before; in the
  browser the page's storage through `store` (localStorage, or this visit's memory when the browser blocks it), each
  save base64 text under its path and the folder's names in `saves/index.txt` (localStorage has no folder listing;
  delete empties the key and drops it from the index). Pause's Save and Quit is no longer hidden in the browser;
  Continue's "5 min ago" clock uses `web_time` there (was 0).
- Already in place (M194/M195), checked again here: Save and Quit above Return to the Menu, the 'Match saved' box,
  the confirm's 'It won't be saved.' note, the tall Continue card over a slim Rematch, the saves list (newest first,
  keeps 5, Resume/Delete, refusals show `Head::check`'s reason), `--pads` re-mapped to today's devices, resume after
  setup (`resume_in_next`).

## Checked

- Tests: `save_game::save_text_round_trips` (base64 against known values and every tail length 0..300, bad text
  refused), `match_flags_drop_the_shot`, plus `hst-play save_game`, `hst menu::`, `hst continue_screen`.
- `tools/webcheck.sh` (the wasm32 `ship` build): clean.
- End to end on desktop (`context/m198/`, `HST_DATA` there): a CPU-vs-CPU `--play` match on Western Prairie saved
  at 0-15 by `HST_PAUSE_SAVE` (`run1/saved.png`); the save's head has the flags; the main menu then shows the Continue
  card "Ashley vs Cody · 0-0 · 0-15" with **Ready** (`run1/menu.png`; before this, the same save showed the
  "Saved without its setup" refusal); Enter on it started the match ("starting the match: --play --court 3 …",
  no resume warning in `run3/log.txt`) with Ashley serving from the ad side, i.e. the second point of the game
  (`run3/a.png`), and the point going on to 0-30 (`run2/a.png`).

## Not verified / not 1:1

- Not against the original: the original has no match saves; menus are ours (PLAN: front-end).
- **The browser path was not run in a browser**: it type-checks (webcheck) and the text encoding is unit-tested, but no
  Save and Quit → reload → Continue in a page was driven (no browser session in this unattended run).
- localStorage quota: a save is ~155 KB, ~210 KB as text; five saves (~1 MB) fit the usual 5 MB, but next to big
  settings/mod data a write can fail; Save and Quit then shows "Not saved" and the reason, as on desktop.
- Saves made before this change (empty flags) stay refused with "Saved without its setup"; they can only be deleted.
- Saved flags include the settings' graphics flags as they were when the match started; resuming replays those
  (M195's behaviour), not today's graphics settings.
- Online saves also record their flags now; M197's lobby resume doesn't use them.

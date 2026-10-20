# B586 — clap emotes: racket away

Task (user 2026-10-20): hide the racket while a character claps (and first: play the clap SFX its game uses).

## Outcome
- First pass shipped a clap sound (each golf game's clap SE as voice program 11). User tried it: those SEs are the
  gallery's applause, not the player's clap. Removed entirely (user 2026-10-20): no clap sound on emotes.
- Who claps: HST's own characters never do. Golf mods' clap takes are the ones made from the source games' clap clips
  (by name: `kuNN_cl`, `rslt_clap_pcNN`, `clap_NN`). Mods repo `tools/rerig/claps.py` (run by `package.py`) marks
  those takes `extras.racket = false`, which the geometric check in motions.py missed for some; 61 mods bumped.
- The character select's emotes never hid the racket: `character::racket_shown` was only scheduled in the match.
  Now also in `main_menu/plate_emote.rs`.

## Not verified / not 1:1
- The lobby fix is checked by build + the plate_emote tests only, not by eye (user tests it live).
- Clap takes are picked by source clip name; a clap clip named otherwise keeps the racket unless motions.py's
  geometric check catches it.
- The original HST has no clap emote; nothing to compare on PCSX2.

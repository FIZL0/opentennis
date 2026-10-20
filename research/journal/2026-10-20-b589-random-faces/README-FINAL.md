# B589 — a modded character picked by Random had no baked face

## What was wrong
Random draws its character and costume as the player readies (`Menu::roll_random`), and the last ready leaves the
select for the confirm at once. The baker (`previews.rs` `bake`) draws the picks only while the select is up; on the
confirm (local) nothing asked for them, and on `Playing` (the launch's up-to-3 s wait for `picks_drawn`) the picks were
pushed onto the queue but the queue's gate (which only lets through the hovered / home / lent / fresh / on-view keys
off the select) returned before drawing them. So a drawn mod (often on a costume nobody had hovered, e.g. costume 7)
waited the full 3 s and the match HUD showed its donor's disc face.

## Fix
`picks_past_select`: on the confirm and while a launch waits, every player's pick (character and the drawn costume)
jumps the queue and passes the gate.

## Checks
- `previews::tests::a_random_draw_is_drawn_past_the_select`.
- Live, `HST_MENU=chars-random`, Enter ×6 (1P and the COMs all on Random), worktree cache missing most mod costumes:
  - before (HEAD build): picks Mifune 0, Taiga 0, disc 1 costume 3, Brad costume 7; the last two never baked, the
    launch waited the 3 s cap, the HUD showed disc faces for the B team ("Guru", "Tier V") over the mods on court.
  - after: Taiga 0, Mel 4, Suzuki 0, Regis 7 (drawn on the confirm 0.3 s after the last ready); launch 1 s after the last
    ready, all four HUD faces the mods' own.
  Logs and shots in `context/b589/run1` (fixed), `run2` (before).

## Not verified / not 1:1
- Online Random computer players (`bot_random`) go through the same confirm/launch path, not run live (needs a peer).
- Front-end only: no original to match.

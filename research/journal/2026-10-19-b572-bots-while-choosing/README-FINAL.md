# B572 — pick for computer players while another local player chooses

User report (2026-10-18, local): at character select, nobody could pick for a CPU player until the last local human
had picked.

Cause: `Menu::driven` (`crates/hst/src/play/main_menu.rs`) only gave a ready human the next computer card once
*every* human was ready, and `roster::is_active` drew a computer card as picking under the same condition.

Change (local select only; online's `online_driven` untouched):
- `driven`: a ready human drives the first computer player not ready, whether or not the other humans are done.
- `is_active`: a computer card is picking once the computers before it are ready and *some* human is ready.
- `preset_coms` (the automatic pick once every human is ready) skips a computer card a ready human is already
  picking for (`held_coms`), so the last human readying doesn't reset it; `carry_pick` doesn't overwrite a card
  another ready human is already on.
- ○ while picking for a computer undoes the last computer pick, else the driver's own pick, never another human's
  (before, it undid the highest-numbered ready card, which could be a human still at work on theirs now).

Tests: new `bots_pick_while_another_human_chooses`; `an_open_seat_is_a_com_card` updated (it asserted the old
wait). `tools/check.sh -p hst main_menu` (221) and `roster` pass.

## Not verified / not 1:1
- Not compared with the original: the front-end is ours to design (PLAN.md), and the original select doesn't let
  a second human and a computer pick at once anyway.
- No screenshot of the new state (two cursors, one on a computer card while another human browses): no `HST_MENU`
  screen sets it up and adding one wasn't needed for the behaviour; the card drawing just uses the existing
  "picking" style, unit-tested through `driven`/`step`.

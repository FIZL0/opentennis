# B611: tap a lobby card's handicap pill (touch host)

Problem: the lobby's handicap was only steppable with L2 / R2; cards drew their pill before `lobby::over`, whose
whole-card seat hit (drawn later, so on top) swallowed any tap on it, and compact cards showed the level as text only.

Change:
- `handicap::pill` also shows for the host on every taken seat's card in the lobby (grey "No handicap" too), and
  hits `Act::Row(seat row, R2)`: a tap selects the seat and steps it up through the existing `lobby_handicap` path
  (`Cmd::Handicap`). The gate is `Menu::lobby_handicaps(i)` (host, not started, no prompt, no countdown, seat taken),
  which `lobby_handicap_seat` now uses too.
- `handicap::lift` moves those pill hits after the seat hits; `lobby::over` calls it after its seat loop.
- Compact (other machines' / computer) cards draw the same pill left of the state pill instead of the
  "… - 1 weight" text suffix.
- `Menu::seat_row(p)` (online.rs): a seat's row in the lobby items.

Test: `handicap::tests::a_tap_on_a_lobby_pill_steps_the_handicap` (every taken seat's pill is on top under the
pointer and a tap sends `Cmd::Handicap(i, 1)`; a guest gets no pill hits). Fails with `lift` removed.
Shot: `HST_MENU=lobby tools/shot.sh` (context/b611/lobby.png): pills on the host's full card and both compact cards.

## Not verified / not 1:1
- Also fixed main's clippy stop in `hst-sim` `swing.rs` smash_mode (negate after the cast; same value, truncation is symmetric).
- Not tried on a real touch screen (phone/tablet); checked by the hit-test test and the screenshot only.
- Front-end, so no original to match (PLAN: menus are ours).

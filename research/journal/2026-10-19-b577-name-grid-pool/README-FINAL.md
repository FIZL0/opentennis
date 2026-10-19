# B577 — name-entry buttons vanish with 4 local players

## Cause
The menu draws into a fixed pool of 3200 UI quads (`main_menu.rs` `POOL`); anything past its end is not shown.
The character select's name pickers are drawn last. Each open picker on its letter grid cost ~920 quads (each of
its 40 keys a two-layer nine-sliced box, 18 quads), on top of 1500–2300 for a full four-player select. With
three or four pickers open (four humans, so after a join) the total reached 5200–6000 quads, and the keys and
Back/Caps/OK of the later pickers fell off the pool's end.

## Fix
- `roster.rs` `hidden_by_name_picker`: while a seat's picker is open, the quads of its card that lie wholly under
  the picker are dropped (the picker box is 124/128 opaque and covers the whole card); anything sticking out of
  the card is kept.
- `names.rs`: the grid's keys are the select's own rounded boxes (`roster::ringed`, 2 quads) instead of the
  nine-sliced box (18).
- `select_fits_the_pool` now also checks the busiest select with all four pickers typing, at every aspect
  (before the fix: 5205–6017 quads; after: under 90 % of the pool).
- `HST_MENU=roster4-names`: four humans, every picker on its grid (shot `context/b577/after.png`: all four grids,
  Back/Caps/OK whole).

## Not verified / not 1:1
- No before screenshot: the overflow is shown by the test's quad count (5788 at 4:3 against 3200) instead.
- Not reproduced with real pads joining live (shots read no pads, P62); the shot sets the same state directly.
- The keys' rim is now the select's rounded-quad style rather than the nine-slice; the same size and colours,
  checked by eye. Menu look is ours (not 1:1 with the original).

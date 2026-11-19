# M195: Continue card and Continue screen

## What landed

- **Main menu**: with saved matches, a tall *Continue* card heads the list ("Carol vs Will · 6-4 4-2 · 30-15",
  "+N more saved"); Rematch shrinks to a slim card under it. The preview shows the faces either side of a mini
  scoreboard (serve dot, set games, point call), a status chip, the words "Picks up on X's serve at P…" and
  Mode / Court / Saved facts. With every card up the pills floor at 32 units and the list scrolls rather than squash.
- **Continue chosen**: one ready save resumes straight away; otherwise the Continue screen opens.
- **Continue screen** (`main_menu/continue_screen.rs`): gold "< Continue" tab, "N saved · newest first · keeps 5",
  lobby-style cards (header in the mode colour, Ready / refusal chip, faces, name, mini board, court, "5 min ago"),
  a navy bar under them (court thumbnail, umpire face, length, serving) and a yellow Resume button ("from 30-15, Carol
  to serve"); an online save says *Open lobby*, a refused one greys it to *Can't resume* with the reason.
  Del / L / △ asks "Delete this saved match?" (Delete / Keep). Pointer: tap a card to select, tap again or the button to
  resume, tab = Back. Legend rows follow each state (legend test covers closed, open, ask, refused).
- **Resume**: the save's own setup flags are relaunched with today's devices put in the menu's seats
  (`launch_flags`/`remap`); `save_game::resume_in_next` hands the save to the next match, applied once `Game` exists.
- `save_game::Summary` now keeps finished sets' games (`set_games`) and offers `call`, `points_line`, `games_line`.
- Aspect: shots at 16:9, 4:3 (960×720), 21:9 (1680×720) and portrait (420×860) of both screens; the cards scale as one
  block, the bar and button pin to the bottom. `cards_fit_each_aspect` checks the rects stay on screen.

Shots: `context/m195/run2/` (all modes, aspects), `run3/continue-delete.png` (after dropping the floating ticker text).

## Not verified / not 1:1

- **No live end-to-end resume**: nothing writes saves until M194 lands, so resume was tested with unit tests and
  sample saves (`HST_MENU=continue…`), not by stopping and resuming a real match.
- **The intro flyover plays on resume**: the resumed match starts through the usual intro; skipping it is M194's side.
- **Rematch after a resumed match** uses the menu's current picks, not the resumed match's: out of scope.
- **No lobby code shown** ("code K7QX"): saves don't store one yet; M197 (online lobby resume).
- **"In its lobby" chip** not implemented: needs the online side, M197.
- **Open lobby** only opens Online with Host chosen and doubles set; it doesn't restore the lobby: M197.
- **Browser builds** have no saves, so no Continue card there (save_game has no web storage).
- **Settings flags** (speed, rules) come from the save, not today's settings; a rules/speed change shows "Rules changed".
- **Front-end is ours** (PLAN: menus don't have to match), so no PCSX2 comparison.
- **Doubles card name** clips at the card edge ("Ashley & Momoko vs Ke…"); it's the shared marquee text.

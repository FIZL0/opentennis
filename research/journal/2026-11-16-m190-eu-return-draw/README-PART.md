# M190: the EU return's extra AI draw is player 1's walk-back roll from an unset formation spot (spec phase)

The problem: at the return's hit message in the European bot match (recorded 230), play's AI generator draws 69 and
the game draws 68.

## Capture
New script `scripts/ai_draws.py`. It patches the game's generator, and every draw from the AI generator appends a
record: the frame, the caller trace (return addresses on the stack), the generator index and the saved registers.
With an AI's address given as a fourth argument, each record also carries a snapshot of that AI and its player.
- Run on `fixtures/m169/eu-bot-start.p2s`, copied to slot 8 of copy 1:
  `tools/pcsx2.sh bash -c 'HST_ISO=fixtures/discs/SCES-54535.iso tools/pcsx2-hst.sh; sleep 15; python3 research/journal/2026-11-16-m190-eu-return-draw/scripts/ai_draws.py 8 1400 context/m190/draws.bin'`.
  It recorded 743 draws over 1418 frames.
- A second run with player 1's AI watched gave `context/m190/draws_p1.bin` (632 draws over 1118 frames).
- The analysis scripts are in `context/m190/` (`ana.py`, `stack.py`, `p1.py`). They map each caller through M169's
  EU→US function matches.
- The per-frame draw counts line up with M183a1's recorded ticks at recorded = frame − 741: 5 at 228, 68 at 230,
  4 at 249, then 1 a tick from 250 to 256.

## Findings
- **The return's message, by player and caller.** Player 0 draws 18: its rally-wait restart, 16 timing draws and
  the reaction. **Player 1 draws 15: timing only.** Player 2 draws 18, like player 0. Player 3 draws 17: its wait
  restart, the re-pick's walk-back roll and 15 timing draws. Play draws 16 for player 1. With a temporary print in
  play, the extra one is the walk-back roll of player 1's own-hit re-pick. Player 1 is the returner, still in its
  receive state.
- **Why the game doesn't draw.** The game rolls the walk-back only when the re-pick moves the spot. The game's
  player 1 holds lane 1, not forward, spot (−2.7425, 11.0) from its set-up for the point (written at frame 47 of
  the capture, long before the serve), and the re-pick leaves it there.
- **Play never sets a doubles player's formation state up.** In play it is still lane 0, not forward, spot (0, 0)
  at that message. So any first re-pick counts as moved, and play's re-pick lands on x = +2.7425. The US point
  agreed only by luck.
- **The fix**, run as a throwaway experiment and reverted: write `Formation::start` (already ported) into each
  doubles computer player's rally state at the point's set-up. With it:
  - the return's message draws 68;
  - recorded 250 draws 1, as in the game;
  - the next difference is at 251, M185's walk-back;
  - hst-play's `fifty_hz` (4) and `doubles` (21) tests still pass.
- **The draws at 249–255** are the contact searches (the reach search, then the tiered search one tier a tick), not
  a walk-back roll or a wait-restart draw. M183a1's best guess (player 2's wait-restart draw made late) was wrong.

`SPEC.md` has the rule for the port session.

## Not verified / not 1:1
- **No port yet.** This is the spec phase. Play still draws 69 at the return's message until the port session lands
  the set-up.
- **Player 1 is the only player whose set-up I checked against the game.** I compared player 1's lane, forward and
  spot with a snapshot of the game's AI state. The other three players' set-up values (from the experiment: 0 lane 1
  back (2.7425, −11); 2 lane 2 forward (−2.7425, −3.2); 3 lane 2 forward (2.7425, 3.2)) follow the same rule but
  weren't compared against a snapshot.
- **Ad-court and non-staggered set-ups** are not exercised by this point. They come from the same set-up rule,
  which `Formation::start` already encodes.
- **Generator state past 229**: the experiment matched the draw counts through 250. It didn't assert the generator
  state there.
- **251 onward** still parts on M185's stand scratch. I didn't look at it here.

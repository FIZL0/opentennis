# B759: decision — no docked preset

Real Steam Deck, docked (run owner, 2026-11-21; nested gamescope 1920×1080, immediate present, 4 CPU players
doubles, mod court, main build with B756, Default 100 %/4x/High):
- rallies about 95 fps, worst frame about 15 ms, GPU about 75 % busy;
- ceremony and result page about 100 fps, GPU 62-72 % busy, CPU-bound around 10 ms.

So the Deck's own preset already holds 60 docked with about 40 % headroom. The user's dip below 60 at match end came
from the B756 cost (the second camera and the confetti), which is already fixed on main. A ceremony-start hitch is
left, queued separately by the owner. Decision (owner's recommendation): docked keeps the Deck's own preset, with no
quality cut. Nothing changes in code, so there's no release note.

The igpu stand-in (`1-BENCH.md`) reads about 2x too slow for the Deck (46 fps there vs about 95 on the Deck at the
same settings), which is why its 67 % pick is not used. The switch code (`Graphics::docked`, a `deck_dock` system)
was on a local branch `wip/B759` and is deleted. It is easy to rebuild if a heavier court ever needs a docked
preset.

## Not verified / not 1:1

- The Deck numbers are the owner's, not measured in this session (no Deck access from here). They were 4 CPU
  players, not 4 humans: the human input path costs no GPU, so this shouldn't change the picture.
- Rendering only: nothing here reaches the sim.

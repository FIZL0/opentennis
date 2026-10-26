# B672 — port: how the walkers take a point (FINAL)

Ported from `SPEC.md` with the decompile locked.

## What changed

- `npc::point_reaction(call, event, over, favoured)`: the gate and the kind. Faults/lets never; a plain point
  always; an out, double fault, net-out or illegal hit only when it won a game or set. A set won → every walker
  cheers from frame 0 (no draw), unless the match is over and the gallery doesn't favour the winner → the match-lost
  clip (6) from frame 0.
- `Walker::react_with(kind)` (mode → react, stagger counter → −1; `react()` is the ordinary kind). `Walker.reaction`
  is new (`#[serde(default)]`). Clip 6 (`anim::LOST`) plays once at speed 1 and holds its last frame until the next
  point's stagger.
- `play/npcs.rs`: the decided point reacts only through the gate; the turn goes to player 0's end for the match-lost
  reaction (else the winners' as before). The match-over handling (`Cheers::match_over`, the manager's resume) moved
  from the next serve to the tick the umpire starts calling the match (`PostPoint::calling_match`), and the walkers
  take the last point's reaction again there (spec §6). The next serve still takes it if the call never came.
- The gallery's pick of clappers still draws at every decided point, reacted or not (unchanged).

## Checks

- `npc::tests::point_reactions`: the capture's table (plain/net-out/game/fault/set/match favoured/unfavoured).
- `npc::tests::walker_reaction_kinds`: cheer and match-lost from frame 0 with no draw, match-lost held on its last
  frame, the ordinary point's one draw unchanged.
- The recorded walker tests (`walkers_animate_like_the_game`, `walkers_face_like_the_game`, rng, the walker oracle)
  stay exact.
- Remaster run (court 10, four CPUs, `--games 1`, seed 1234; shots in `context/b672/port/`, `sheet.jpg`): at the
  match point every walker in view claps with a CLAP mark (the unpicked one too), as in the original's capture.

## Not verified / not 1:1

- The match-lost reaction (clip 6, turned to player 0's end) was not seen in a remaster run: a run with an idle
  human on team 0 (`--pads -,-,k,-`) stalled (that seat never served), so it is checked by the unit tests and the
  capture's numbers only. No PCSX2 screenshot of the original's set end was taken (it is ~9 minutes into slot 5);
  the comparison is against the earlier capture's per-walker clip log and its CLAP-mark screenshots.
- The second reaction is taken when the umpire starts calling the match; the original's match-over message comes
  ~110 frames after the last point. The remaster's timing for that moment differs by its own post-point flow.
- One-player mode (spec §2.1) is kept as it was (clap from frame 0, a mark, no draw) but no remaster match has one
  player; the instant-replay pick carry-over (§5) is not ported (the remaster's replay doesn't re-run the walkers).

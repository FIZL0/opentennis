# B40g2 round 2: more offbeat rules (user 2026-10-21)

On `concept/offbeat-rules` (worktree `../opentennis-offbeat`), three more rows in the confirm screen's Rules box
(Select / C); ✕ cycles a row's choices:

- **Player size** Normal / Tiny (0.45×) / Big (1.8×): drawing only, the sim's reach and contact points stay, so a
  giant's racket can miss the ball visibly and a tiny one reaches far. Markers and name tags follow the head height.
- **Player speed** Normal / Fast (1.6×) / Slow (0.6×): the mod run-pace scale on every player's speed stat; the CPUs
  plan with it.
- **Sweet spot**: every rally hit, serve and dive gets the sweet grade at offset 0 and no timing/height error, so no
  mis-hits and every hit shows the Sweet balloon.

Try: `git switch concept/offbeat-rules`, then the confirm screen, or `--offbeat tiny,fast,sweet` (also `big`,
`sluggish`). Test `offbeat::tests::sweet_spot_and_fast_players`: every non-dive hit is sweet, run speed ×1.6.

Next (the user decides): should reach scale with size (a giant reaching further would change the sim); tune the factors.
The shots in `context/b40g2/sizes/` showed no court in a `--play` shot from the worktree, which these rules don't
cause; not chased.

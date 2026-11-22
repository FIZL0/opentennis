# F3: end screen headers as the concept

The concept's 'End screen' tab: the score board's stripes read the character (Carol, Jun…) beside the face; the
stats header reads who plays (1P / COM with its level).

- `plate.rs`: each player's second word image is now the character's name in the plate lettering (white, team-colour
  outline, 22 px), not the navy pill. `place` sets one word image in its room beside the face: narrowed down to 70 %,
  then shrunk about its middle.
- Score board (`board.rs`): the character's name where the slot label was, centred on the ring; no pill.
- Stats header (`match_stats.rs` `plates`): the player's name (a named human), else 1P..4P / COM, with the game's own
  rank art under a CPU's "COM" (the "Lv 5" / "Hopeful" rows); no character pill. The pause board (F2) reuses both.
- The plate pool (`name_tags`, 4 per HUD): since B764 the end screen draws its own word images (`plate::Words`) and
  emits no slot-label quads for the pool, so no pool change was needed; `name_tags.rs` is untouched.

Checked: `tools/check.sh -p hst-play match_stats` (23 pass). Shots in `context/f3/run1/` (`HST_STATS=4 --shot-at 14
--play --stage 10`): `end-names.png` (16:9, P2 human named "Ryan": stats header "Ryan", score board "Jun"),
`end-43.png` (4:3, unnamed human: "2P"), `end-219-singles.png` (21:9 singles, Big Chief vs a named Miranda) — beside
the concept tab they read the same: names beside faces on the score board, 1P / COM Lv on the stats board.

## Not verified / not 1:1

- "COM Lv5" is the game's own rank art under "COM" (its rows include words like "Hopeful", "Tier II"), not the
  concept's typed "COM Lv5" text: B764 asked for the game's existing parts, and the rank row is the original's.
- Long character names (Big Chief) narrow then shrink in the 37-px room beside the face, so they read smaller than
  short ones; in singles the plate has spare width the room does not use.
- `--names ",Ryan"` through `tools/shot.sh` lost the leading empty name (hyprctl exec quoting), so the named shot
  names every player "Ryan"; only humans show names, so it shows the same as ",Ryan" would.

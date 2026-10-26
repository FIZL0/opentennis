# B678 spec: the gallery's "This is it!" line

The user heard a spectator say "This is it!" in the original. It is gallery bank program 9 (two takes, keys 0 and 1;
every court's bank has them, and the bank's program 2 is a silent placeholder the game never plays). The game says it
only on courts 1, 4 and 6 (the game's court numbers, the same ones in the gallery bank names `gal..01/04/06`), when a
human is about to smash a lob: they have run to within 3 m of the yellow smash marker's point.

## When it plays

Checked each tick by the gallery's manager, in its reactions step: after the cheer step, before any other reaction
(the "Reactions" step in `research/journal/2026-10-07-n3-audio/12-GALLERY-FINAL.md`). It is skipped on the ticks the
whole manager doesn't run (rain, paused by the deciding-set figure, the camera's match/set-point hold). All of these
must hold:

1. The match has two or more players (the same check as the other reactions).
2. No gallery line is still sounding (see *The sounding line* below).
3. No gallery reaction is waiting (no applause, groan, shout or match-end event pending).
4. The court is 1, 4 or 6.
5. The "checked" flag is clear (see *The once-per-shot flag*).
6. The smash marker has placed at least one point for the ball in flight.

When 1–6 hold, the flag is **set first**, before the distance test. Then, for each placed point in the order it was
placed: if the player it was placed for is within 3.0 m of it (horizontal distance in x and z, the player's current
position), the line plays and the check ends. So the distance is tested only on the **first** tick that conditions
1–6 hold after a hit. If no player is close enough on that tick, there's no line for that shot, even if they get
closer later. A tick where 2 or 3 fails leaves the flag clear, so the test can still come later in that shot.

## The play

- Program 9, key alternating 1, 0, 1, … (a counter the gallery keeps for the whole match; the first line is key 1).
- From one stand picked at random: one draw from the court generator (the gallery's own `roll`), stand
  `(draw >> 16) & 7` of the gallery's eight bearings (`STANDS`), as the applause shouts pick theirs.
- Volume 0x4c (the applause/groan volume), normal pitch, not placed (bearing only), like the applause.
- That draw is the only random number it uses, and only when the line actually plays.

## The once-per-shot flag

Cleared at the end of every manager tick on which a new hit has been registered (the match's hit count changed and
there's a hitter). It is cleared after that tick's reactions, so the hit's own tick still sees the old flag. It starts
clear at the start of a match.

## The sounding line

The gallery remembers the line it just started until it stops sounding. On every running manager tick, before the
cheer and reactions, if the remembered line is no longer playing, the gallery forgets it. While it is remembered:

- the "This is it!" check doesn't run (condition 2);
- the reactions step drops any pending reaction **at once** (the applause, groan or shout that a point started, even
  one still counting down its delay) and does nothing else that tick. The exceptions are the after-match cheers and
  sigh (events 7, 8, 9), which wait for the line to finish. The rolling cheer is unaffected: a point won off a smash
  just after the line still gets its cheer, but not its applause.

The match-end ovation/groan (programs 3/4) is remembered in the same way (only the last of its three stands' plays
counts), so a new reaction during it is dropped too, except the cheers/sigh that follow it.

When the manager doesn't run (rain, or the camera's match/set-point hold), it forgets the line and drops the pending
reaction (the existing `idle`).

"Still playing" means that voice in the gallery's sound slot is still sounding, so it lasts as long as the take.
Program 9's takes run about 0.6–0.8 s on the courts checked (court 4: 0.78 s and 0.61 s at 12 059 Hz). The take's own
sample length in ticks is the faithful stand-in when the sim can't ask the mixer.

## Which points count

Only points the original places count: its smash-marker search runs only for the **human** players on the receiving
team, with their own smash heights, from the serve return on (`smash_marker::receivers` without our B507 additions).
Our marker also marks lobs onto all-CPU teams and caps the heights at a low lob's peak. Those points are ours, for the
drawn marker only, and must **not** trigger the line, or the court generator would be drawn where the original never
draws it. Each point remembers the player it was searched for. The list empties when the ball in flight bounces (the
marker goes) and when a new strike starts a new search.

## Already right

Nothing in our port plays program 9 today (it's listed as left out in `sound.rs`'s `Gallery` doc and the N3d4
journal), and the bank already loads it. No other gallery sound is missing: applause 0, groan 1, match end 3/4,
shouts 5/6, after-match 7/8, rolling cheer 10 all match the game's triggers.

## Checks

- Unit (hst-sim): the flag's once-per-shot rule (set before the distance test; cleared by a hit after the reactions),
  the key toggle, court filter, the stand draw, and that a sounding line drops a pending applause but not event 7.
- Replay/bit-exact tests must stay unchanged: all-CPU matches never place original points, so they never draw.
- By ear: court 4 (or 1/6), a human receiving a CPU lob, running under it: "This is it!" once, then no applause if the
  smash wins while it's still playing.

## Open questions

none. (Within a tick: whether the path search runs before or after the gallery's manager isn't pinned. Use the
points as they stand after this tick's search; presentation only.)

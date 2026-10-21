# P117 — spec round 2: the special gate inside the court pull

Answered SPEC.md's open question (C.1). The pull's special gate isn't read live from the player: the match keeps a
per-player copy of each player's last lock grade and timing offset, refreshed on the frame a swing locks (skipped
for the 999/9999 "no timing" offsets), and the gate reads the striker's copy (the strike sets the last-hitter
index first). The port phase's oracle never set those copies, so the gate stayed shut there.

Live check: `research/p117_latch.py 5 2400` (slot-5 bot doubles, PCSX2 copy 5) logged the copies, the players'
own lock grade/offset/ready flag and the hitter index each frame: every hit (22 of 22) had copy == the striker's
current lock, and the lock came 5–8 frames before the contact. So `special = shot::special(striker) && effect row
non-zero` on the retail disc; clean lobs heading across keep unscaled margins in the original.

Next: port phase — replace the strike's `special = false`, and extend the oracle test by setting the striker's
latched copy (location in the probe script) instead of the debug switch.

## Not verified / not 1:1

- The latch == live lock equality is from one 2400-frame bot game (22 hits), not every swing path (human swings,
  serves, dives); a swing that makes contact with a 999/9999 offset would read a stale copy, and none was seen.
- No clean lob or 3/8 slice heading across has been recorded with its pulled target yet (the port phase's job).

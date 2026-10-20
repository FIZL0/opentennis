# P11m2: the AI generator's draws before the first point (spec phase)

This was the spec session. It read the decompile and wrote `SPEC.md`, but no Rust. The next session ports from it.

## How it was checked

- `research/p11m2_intro_probe.py <name> <slot> [follow s]` (under `tools/pcsx2.sh`) loads a Confirm Settings state,
  starts the match, and logs on every change: the AI generator's index, the match phase, and each player's
  state/AI-state bytes. It runs until P0 can serve, then optionally serves and follows on.
- The states came from P11m1's Confirm Settings saves (copy 4's slots 8/9), put into copy 3's scratch slots:
  singles Ashley (human) vs COM Cody, and doubles Ashley + COM Jun vs COM Cody + COM. A CPU-vs-CPU singles state
  (COM Ashley vs COM Jun, Queen's) was built from the singles one.
- Captures: `context/p11m2/{singles,doubles,doubles_follow,cpu_singles}.json`. Each one matches the counts in
  `SPEC.md`: setup leaves 36, the intro resets add 18 per AI object, then roles add 17 / 0 / 1. The first point
  adds 18 per AI object, the second 17.

## Found on the way (in the spec)

- The app never resets a human's AI object (a human beside a CPU partner); the game does at every point.
- The game draws the first point's net pick when the partner has an AI object; the app requires the partner to be a CPU.

## Not verified / not 1:1

- Doubles with two CPUs on P0's team, or a CPU serving first in doubles, wasn't captured. The counts follow
  from the same rules (captured for singles CPU serving, and for doubles rally/receive roles), but they weren't
  observed directly.
- Matches with a cut-scene flag set during the intro (the game then skips the AI and does partial resets) weren't
  seen. The flag stayed off in every capture, and the app has no such path.
- Only draw counts were checked, not the values drawn. The values feed AI state that the first point's reset
  overwrites; the port session should compare the AI generator's index (and state words) at the first point to
  these captures.

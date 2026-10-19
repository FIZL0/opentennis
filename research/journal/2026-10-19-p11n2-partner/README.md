# P11n2: doubles partner logic (spec phase)

The spec is in [SPEC.md](SPEC.md). Status: the spec is done (`--spec`), and the port session goes next.

## Findings
- The task line says the app's "nearer by x" stand-in (`bot`'s `mine` filter and its call-out) decides who plays a
  doubles ball. That block can't run in a real match:
  - P11k3 routes every receive and rally frame of a computer player through `doubles_ai::step`;
  - on every other frame, `bot` returns early or has no interception plan.

  Who plays the ball is decided by the ported routines: shot records, give-way, follow, middle pick and voice
  (P11k2), plus the claim and the human record keeper (P11k7).
- Three inputs to that logic aren't wired in the app:
  - the partner's movement state, which gates the give-way voice and its AI draws;
  - the player's own one-frame miss mark, which posts a "missed" record so that a following partner takes over;
  - the claim's exemption from keeping 1 m apart while stroking. The app's contact-step ponytail ("partner check
    skipped for the server") is this rule.
- Scan of `context/fixtures/match_s05.bin` (four CPUs, 26 400 frames):
  - teammates are never both locked after the first set-up;
  - both pressing at once happens 3 times;
  - 6 miss marks, each 1 frame, all from presses that found nothing;
  - teammates never come within 1.52 m, so keeping apart isn't exercised.
- `1p3goodcpus.bin`: 1 mark; teammates sit at 1 m only while running or standing.

## Method
- Read the doubles rally routines' swing step, the give-way voice, the claim setter and reader, the stroke-state
  movement update, the miss routine and the contact search's resets, and their callers.
- Checked the ported Rust (`hst_sim::rally`, `doubles_ai.rs`, `play.rs`) for which parts are already wired.
- Scanned the two fixtures with a throwaway script in the scratchpad. It isn't committed: it reads only the
  fixtures and prints counts.

## Tool note
`research/xref.py` fails inside a `.claude/worktrees/…` checkout: it builds the `.funcs.tsv` path with
`path.replace('.c', …)`, which also rewrites the `.c` in `.claude`. Run the main checkout's copy, or fix it to
replace only the suffix. `research/fn.sh` works once `context` is symlinked into the worktree.

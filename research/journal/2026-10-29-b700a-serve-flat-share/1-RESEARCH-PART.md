# B700a research: CPU serve flat share against press lateness (PART: the port session continues from SPEC.md)

Question (from B700): in the original a late CPU serve press seemed to come out flat far more often (2/2 in B700's
run, 10/15 in the will-curve notes), though neither the AI's kind roll nor its aim grid reads the press frame.

Answer: the press frame has **no effect** on flat. Shown by intervention on PCSX2 copy 1: the same serve was
replayed from a save at the toss with only the serve timing error forced (−4..4). The swing kind, the contact stick,
the button after contact and the ball kind never changed (15 serves). Only the timing offset moved. The serve contact
reads the stick the AI's serve state writes on the aim frame, which is the contact frame itself. Every other swing
frame the AI's stick is centred. The rally routine takes over only after the stroke ends, 30+ frames after contact.
Spec: `SPEC.md`.

## Tools (research/, kept)
- `b700a_serve_stick.py <slot> <s> <out.bin>`: per-frame trace of a CPU serve from press to launch (AI and swing
  countdowns, stick word, button, aim, ball kind). Run: `context/b700a/run1/`.
- `b700a_err_poke.py <slot> <serves> <scratch> [errs] [mash]`: the intervention. Runs: `context/b700a/poke2/`
  (6 serves × 8 replays) and `poke3/` (9 serves × 5, mash 0.6, two flats). `poke1/` was a broken try: the toss
  draws the timing errors again, so a poke made before it is overwritten. The tool now re-pokes every frame until
  the pick.
- `b700a_flat_share.py <press logs>`: flat share of topspin swings by real offset, from `b700_serve_press.py` logs
  (`context/b700a/runs/press_*.log`, five mash periods, plus B700's `context/b700/run1/press.log`).

## Numbers
Observational (52 topspin swings, 4 repeated): offset 0 flat 3/31; −1: 2/7; −2: 2/4; −3: 1/3; −4: 4/7. Late
taken together: 9/21 (Fisher p ≈ 0.02). The intervention rules out a causal link, so this is chance or the generator's
sequence. The will-curve 10/15 was a selection effect: its "offset" column was the depth error, which flat
(forward-stick) serves show.

## Not verified / not 1:1
- Doubles CPU serves: not recorded live (the decompile's doubles serve state has the same aim step).
- Whether the port's draw order on the AI generator matches between the press and the contact. The original draws
  nothing there (the kind roll is identical at any press frame), and the port's aim tick should be the contact
  frame. Left to the port phase as a test (SPEC.md *What the port should check*).
- Why the observational share leans to flat when late (chance or generator sequence): not settled. It doesn't change
  the rule.

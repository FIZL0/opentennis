# P11n1 — AI ball-path record (2026-10-19)

Spec phase: `SPEC.md` (how the game builds the predicted path the CPUs read, the messages that start or stop it, how
the predictor is reseeded, what the AI reads). Next session: port into hst-sim and feed `play/doubles_ai.rs`.

- `tools/record_live.py --path` writes `<out>.path`: the record's header, the claim, the AI copy's stamp/count and
  the last up to 32 entries per frame.
- Run 1: `HST_PCSX2=2 tools/pcsx2.sh python3 tools/record_live.py 5 1500 context/p11n1/run1/live.bin --path`.
  1500 frames, none missed. It confirms the growth rule (15 a frame to the second contact, then top-ups when fewer
  than 15 are left), start +1 a frame, entry 0 = the live ball at the hit with y negated, the hit frame ends at
  end 15 / start 0, mode 0 on serves and 1 in rallies, and inactive at the point's end.
- Not in run 1: a net touch, an out ball (gap), the 600 cap. SPEC.md *Recordings* says how to get them.

# B40g1 spec phase

Read the decompile for the handicap: the player set-up, the run-speed readers, and the weight models (create,
rebuild, per-frame placement). Wrote `SPEC.md`. The next session ports from it with the decompile locked.

Findings, in short:
- Handicap 0/1/2 sets size 100/80/70. Only the run speed reads it (the human run step and the CPU run estimates).
  `hst_sim` already has the factor. The app passes 100 everywhere.
- One weight per handicap level, slot 0 on the right calf and slot 1 on the left. The model depends on the character:
  JJ, Big Chief, and Kent in costumes 5–9 get their own; everyone else gets the default.
- Each character/costume row has an offset and a scale. Mirrored (left-handed) bodies use the other calf, with its
  x axis flipped. A weight is drawn only on frames when the body is drawn.

## Not verified / not 1:1
- Everything here comes from reading the decompile. Nothing was checked in lock-step or captured in PCSX2 in this
  session. The port verifies it: the oracle for the speed, screenshots for the weights.
- The default placement matrix the mirrored case starts from reads as zeros in the ELF dump (an uninitialised data
  section). The spec assumes identity: the model constructor gets the same matrix, and with zeros nothing would
  draw. The screenshot check covers it.
- Where a lefty's weights land in the original, given how it draws mirrored bodies, is described as the math does it.
  The port is told to match the visible result by eye.
- Whether the weights cast a shadow was not looked up. It's rendering, judged by eye.

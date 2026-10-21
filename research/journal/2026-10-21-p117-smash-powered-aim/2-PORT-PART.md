# P117 — port phase (A, B, D done; C's gate back to the spec)

Ported from SPEC.md with the decompile locked.

## What landed

- **A, smash mode and blend:** `swing::smash_mode` (0, or −trunc(((|z| held to 6.4..11.885) − 6.4) / 5.485 · 10
  + 0.5) for a plain smash off its timing) and `swing::smash_blend` (Serv POW against SM LOW POW, −0 kept).
  `timing::smash` returns the blend next to the scatter; `Game::smash_scatter` carries it to the strike.
  `tests/timing.rs` `timed_smashes_like_the_game`: the mode equals the game's stored mode (+0x3ed8) for all 29
  recorded smashes (19 scaled, 15 low), and every launch target stays bit-exact with `low` from the blend.
- **B, low margins:** the strike pulls a smash with a negative blend by the low pair. Oracle
  (`hst-oracle/tests/court_pull.rs`, `court_pull`): 20k pulls of every class/kind/blend sign (−0 included),
  singles/doubles, all characters, bit for bit, ~400 of them moved by the low margins.
- **D, smash spin with a blend:** kept the base `smash_spins`; `tests/shot_params.rs`
  `smash_down_variants_keep_the_base_spin_group` proves every character's dw1 smash record has the base spin
  group bit for bit, so the mix is a no-op on the disc.

## C, special (unscaled) margins: open

The oracle says the pull's special gate isn't the hitter's lock grade (SPEC.md *Open questions*). Tried, from the
slot-5 RAM dump: every player block's (`*(gm+0xa8+4i)`) lock grade +0x3ee8 and offset +0x3fa0 random, `a0` = player
0's block, the hitter index 0x423058 set to 0..3 with a clean grade in one player at a time; then writing 1 (and 2)
into single bytes of all four player blocks (0..0x4400, with grade 1 preset), the match object (0x400), the live
ball (0x290), 0x3f0000..0x422f80 and 0x422f84..0x427000 (skipping the gm pointer and the hitter word, which crash)
and the scratchpad. Only 0x410870 (the debug switch) and the margin table itself change the result. With the game's debug switch forcing the gate, the effect-row rule (rally slice bend / lob curve non-zero)
matches 20k pulls (`court_pull_forced`). The strike keeps `special = false`, which is what the game does in
every state the oracle reached. Back to a spec session to find the gate's input.

Oracle notes: an aim outside the court (singles sideline in singles, doubles' in doubles, or past the far
baseline) sends the game's pull into a report path (a memset with an MMI op the interpreter lacks, then a kernel
call), so the generated aims stay in the court; play never aims outside it.

## Not verified / not 1:1

- **C's gate** (above): unknown, so a clean lob or a character 3/8 slice heading across never keeps unscaled
  margins in the port. Whether the original ever does in play is the open question.
- **The blend's magnitude:** only its sign is used (B; D is a proven no-op), and the sign is checked through the
  recorded mode. No recording has a low smash whose aim reaches the low margins (all 15 recorded low smashes land
  inside them), so B rests on the oracle, not a recording.
- No new PCSX2 recording was made (no deep, mistimed smash at a corner or clean cross-court lob near a line).
- The body-shot (Body DWN) addition to the smash mode is left out, as the spec allows: a smash is never played
  from the body-shot motion.

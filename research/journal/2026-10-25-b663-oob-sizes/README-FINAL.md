# B663 — Out of Bounds characters too big

## Cause
The mods repo (`../opentennis-mods`, `tools/rerig/sizing.py`) sized every Out of Bounds character by its Bip01 (hip)
height: profile height in cm × HST's metres of Bip01 per cm, calibrated on Gloria. Suzuki, Fernando, Felipe, Bloom,
Maruyama and Nick have cartoon proportions (short legs, big heads: top/Bip01 2.0–2.3 against Gloria's 1.89), so a
hip height right for their cm put their head-to-foot height 8–23 % over it. Suzuki stood 1.93 m against HST
Suzuki's 1.57 m, Fernando 2.19 m against Kaito's 1.83 m.

## Fix (mods repo, feebd5a)
- `sizing.py`: Out of Bounds (`TOP_SIZED`) is sized head to foot: cm × HST's median metres of top height per cm
  (standard bodies, by sex), the game factor on Gloria's top; each character's Bip01 then follows from its own
  proportions and head factor (head shrunk about the neck). `TWINS`: OOB's Suzuki is HST's Suzuki, so he takes
  HST Suzuki's top height (1.5741 m) outright.
- `package.py`: every game's Gloria keeps HST's Gloria as donor (the new sizing would have moved OOB's to Miranda).
- `gag_stats.hst_xb`: HST's disc extract is read from `../opentennis/fixtures/xb` (it moved from `context/xb`; without
  it a repackage silently drops every params override and motion length).
- `batch.py oob` + `package.py oob` rerun; all 17 `oob_*` mod.json versions bumped (+1).

Heights after (top of mesh, m): Suzuki 1.930 → 1.574 (HST Suzuki 1.574), Fernando 2.186 → 1.878, Felipe
1.989 → 1.899, Bloom 2.002 → 1.878, Maruyama 1.949 → 1.694, Nick 1.914 → 1.673; Dino 2.011 → 1.899, Jasmine
1.690 → 1.594; the rest move ≤ 2 % (Gloria 1.655 = HST Gloria). Checked by eye: `--inspect 13` vs
`--inspect …/oob_pc13_suzuki` (context/b663/run1/pair.png), same height.

Side effects of the new Bip01 heights (donor = nearest Bip01 among same-sex standard bodies): Fernando's donor
Will → Cody, Jasmine Ashley → Carol, Sonia/Kate Gloria → Miranda, Anya Miranda → Gloria (ai_row too),
Alex Ashley → Gloria; their params base and reaction lengths follow the donor.

Test: `mods::tests::oob_hair_swings_and_settles` now runs 6 s instead of 4 s: the smaller Jasmine's hair settles a
little slower (0.63° off rest at 4 s against the 0.5° bar), same bar kept.

## Not verified / not 1:1
- Not uploaded to the mod store: the bumped mods sit in `out/mods`; publishing them is the user's step.
- Sizes are judged against the stock roster by measured mesh height and one viewer pair, not in a match.
- `--inspect` with several characters drew them all on one spot in this build (the shots in context/b663/run1);
  not looked into, unrelated to sizing.

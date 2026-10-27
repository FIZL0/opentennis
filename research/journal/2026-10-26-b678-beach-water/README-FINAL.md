# B678 Laguna Beach Resort water looks still

The user meant the **court mod** (Get a Grip's beach resort court, `gag_c04_resort`), not only disc court 2
(user, mid-task: "if you made improvements to the stock one keep them too but make sure to tackle the modded one").

## Court mod: the sea scrolls
- Get a Grip's court04 data has nothing that animates its water: no `.tat`/`.i3m` in court04.xb. The `@water` in the
  texture names is a ball-hit surface tag, like `@kusa`, `@rock`, `@wood` and `@clay`. A table of the i3r texture records'
  flags over every court shows no field that only animated materials carry. The PSP program (encrypted EBOOT) must
  move it, if it moves at all; there's no Get a Grip disc image here to look.
- So the speeds are by eye. New court standard field `scroll` (`{"<material>": [u, v]}` texture repeats a second;
  `../opentennis-mods` standard §2 and the exporter's `SCROLL` table, pushed as e49c1e0). HST reads it in
  `mods_court.rs` (`Court::scroll`, checked: two numbers within ±10). `court_mod.rs` collects the matching
  materials into `Kept::scroll` while building them, and `slide` sets `uv_offset` from the clock each frame,
  with whole repeats dropped.
- c04: base sea (0.006, 0.004), foam layer (−0.004, 0.007), additive caustics (−0.009, −0.005) both variants;
  sheets are ~70–110 m a repeat, so ~0.5–1 m/s drift. Built mod's `mod.json` gets `scroll` + `"version": 2`
  (out/ isn't in git; re-export or upload it as is).
- Checked: `tools/shot.sh … --play --court-mod …/gag_c04_resort`, two key shots 2 s apart: sea differs (mean
  |Δ| 12–26), island/deck/sky identical. (`--court-mod` needs `--play` or the plain layout court is drawn.)

## Disc court 2: the surf's `.MOR`
- The hole's `.MOR` (3 targets of the foam sheet `000shibuki`, horizontal offsets up to ~9 m) is now played:
  `hst_sim::court_anim::CourtAnim::with_morphs` (weights per target on the model's clock, looping) and
  `hst_base::court_anim::Morph` (rest + Σ w·d written through `vertex_patch`, box over every reach, `NoAutoAabb`).
  Offsets come from `stage::morph_offsets` in `mesh()`'s vertex order. Test `morph_weights_follow_the_mor`.
- Checked: `--stage 2` shots 3 s apart: the foam ring moves from offshore onto the beach.
- The MTA's `shibuki111/222` still bind no material (nothing to do); the open sea has no track.

## Not verified / not 1:1
- Mod scroll speeds and directions are a design by eye; Get a Grip itself was not looked at running (no disc
  image), so whether/how its sea moves is unknown.
- Court 2 surf: not compared frame by frame with PCSX2 (presentation, by eye only); the start phase (from the
  hole's spawn) was not checked against the original. Whether the stock open sea moves in the original was left
  open: PCSX2 shots in context/b678/q_* hinted at it but weren't conclusive.
- The rebuilt `gag_c04_resort` (version 2) still has to be uploaded to the mod store.

Follow-ups: B678a (plants' `.MOR` sway on courts 2/3/4/6/9), B678b (other Get a Grip courts' water scrolls).

## Blocked on an unrelated test
`tools/done.sh` failed twice on `hst-base mods::tests::oob_faces_all` only: `oob_pc18_toro` in the installed
mods folder (`mods/hst-mods` → opentennis-mods `out/mods`) "plays [] … no target" for every face clip. That mod was
rewritten at 14:44 by another session (opentennis-mods has uncommitted `tools/rerig` edits). B678 touches neither
mods.rs nor character mods. Unblock once that mod's rebuild is finished (rerun `tools/done.sh B678`).

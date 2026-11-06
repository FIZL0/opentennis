# B702: "This is it!" at the wrong pitch

## What the original does (PCSX2, copy 1, slot 4: court 4, bank `galsg04b`)
`b702_rec.py 4 3600 …` (here) under `tools/pcsx2.sh`, with vpad ✕ every 1.5 s: the opponents' strokes are forced to
lobs and P1 is moved under each lob's descending path, so the gallery's "within 3 m" test passes. It logged every
sound-ring key-on. Six lines in 3600 frames (vsync 7961–10976):

- Each line is one key-on on the gallery's bank: ADSR 80ff/9fe0, key 0 = pitch word 4630f940 (root 70, note 48,
  fine −7), key 1 = 4731f940 (root 71, note 49), scale 0x1000 (range 12). That's 12 059 Hz, the bank's own rate.
  No later pitch command for the voice while it sounds.
- The keys go 0, 1, 0, 1, 0, 1.

So the pitch *words* already matched: our mixer keys the line on with exactly those words, ADSR, sample start and
SPU pitch register (new test `hst-base audio::tests::gallery_this_is_it_pitch`, against the recorded words).

## The bug: the take order
- The line counter: with `states`, scratch states 8/9/8 were saved right after three lines, and EE RAM was diffed
  against the slot 4 load. The only 0/1 that follows the lines is in the gallery object (the pointer at 0x43b1c0):
  0 at the load, 1 after a key-0 line, 0 after a key-1 line, 1 after the next key-0 line. Slots 3 and 5 hold 0 too
  (slot 5 is court 10, which never says the line, so 0 is the value a match starts with).
- So the original plays the key the counter holds and then flips it: **the first line of a match is key 0**. Our
  port (from the B678 spec, "the first line is key 1") flipped first, so it always played the other take.
- The two takes are different voices with quite different pitch. Median F0 of the decoded takes (autocorrelation)
  key 0 / key 1: `galsg01a` 431/377 Hz, `galsg04b` 377/355, `galdv04a` 365/548, `galdv01a` 188/301,
  `galdv06a` 268/355. Hearing the other take sounds like the wrong pitch.
- Also checked: every court 1/4/6 bank's program 9 has its own samples, 22 semitones under root like all gallery
  takes (12 059 Hz). The rolling cheer (random pitch 0.875–1.25) shares samples with program 9 only on the other courts,
  where the line never plays. The cheer's random speed doesn't carry to later gallery plays (applause after cheers
  is at 0x1000 in `hits_s04`).

## Change
- `hst-sim/src/sound.rs` `Gallery::this_is_it`: plays `line`, then flips it (first line key 0); `sounding` takes the
  played key's length.
- Tests: `tests/sound.rs` `this_is_it_*` keys 0, 1, 0 (lengths swapped to match); `gallery_this_is_it_pitch`.
- Correction to `2026-10-26-b690-gallery-this-is-it/SPEC.md`: the first line is key 0, not 1.

## Not verified / not 1:1
- Not compared by ear (unattended, the PCSX2 copy is muted); checked by the SPU command words and the take order
  instead.
- The counter's start value is taken from two states that never said the line (slots 3 and 5 hold 0), not from a
  freshly started match, so a reset elsewhere (e.g. between games) isn't ruled out. Within the recorded stretch it
  kept alternating across points.
- Only court 4's `galsg04b` bank was recorded; the other banks use the same code path.

## Aside
`tools/patches.py` failed on main's `third_party/bevy_pbr/src/lib.rs` (b7da0da0's comment `// opentennis: …` doesn't
match the patch marker), which stops every `done.sh`; the comment now reads `// opentennis patch: …`.
Also from the same split-screen work: `hst-play` `play::split_view` tests panicked in `twin_view::shared_cascades`
(`Res<DirectionalLightShadowMap>` missing in the headless test app); it now takes the resource as optional and
returns without it.

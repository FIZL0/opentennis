# B604 — a damaged ISO must fail, not crash the disc reader

From the 2026-10-21 repo audit: `Iso::walk` length-checked each directory record but then indexed the name length
(byte 32), the name bytes and took whatever directory size the disc claimed as an allocation.

## What changed (`crates/hst-data/src/iso.rs`)
- A directory record shorter than its fixed 33-byte part, or whose name length runs past the record, is
  `InvalidData` ("bad dir record"). Everything else `walk` reads from a record (extent at 2..14, flags at 25) lies
  inside those 33 bytes, so it can no longer index out of range.
- A directory extent over 1 MiB is `InvalidData` before anything is allocated (the retail disc's directories are a
  few sectors).
- `read_at` checks `offset + length` against the end of the source before allocating, so neither a directory nor
  a file entry (`Iso::read`, sizes up to 4 GB from the disc or a pack index) can allocate past what is really there;
  it returns `UnexpectedEof`.
- Tests: `iso::tests` builds tiny images with a short record, an oversized name length, a 4 GB directory, an
  extent past the end, and a file entry past the end.

## Fuzz target (`fuzz/fuzz_targets/disc.rs`)
Now also drives: `Iso::from_source` (ISO or pack) → read every file → `texhash::disc_textures`; `mor::parse` (MOR
and UVA widths); `snd::Bank` (parse, key-ons, programs, tones, set volumes); `snd::Sequencer` (up to 256 frames);
`layout::plants`, `layout::entries`, `layout::cloud_count`; `noi::parse`; `texhash::tim2_names` on decoded TIM2s.
Seeded locally from a sample of the user's extracted disc files plus a hand-made ISO and pack (seed corpus kept
in the scratchpad, not committed).

## What the fuzzer found
`cargo +nightly fuzz run disc` over the seed corpus (built with debug assertions, as cargo-fuzz does by default):
- `snd::Bank::key_ons`: the frame counter overflowed on a sequence of huge deltas → it now ends as `None`
  (test `snd::tests::a_damaged_sequence_with_huge_deltas_ends_without_a_crash`).
- `snd::Sequencer::frame`: a loop with no time in it (or an infinite tempo) spun forever inside one frame and grew its
  output until out of memory → more than 4096 messages in one frame ends the track as damaged (retail BGM has a
  handful per frame; test `snd::tests::a_loop_without_time_ends_the_track`).
After those, 600 s / ~1.0 M runs with no crash, OOM (2 GB) or timeout.

## Not verified / not 1:1
none — no gameplay behaviour involved; the readers only reject input they used to panic on.

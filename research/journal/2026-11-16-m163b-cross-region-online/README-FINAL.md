# M163b — cross-region online play

## What landed

- `hst-data::digest`: a disc's **gameplay digest**, one XXH3-64 over what feeds the match:
  - the game program's tuning tables (`exe::Game`), read in the US layout, so a release that only moves them digests the same. Left out: HUD colours, ball/crowd sounds, cheer sprites, rain/weather looks, bounce looks, player lighting, and the umpire word timing for languages the match doesn't read (it reads language 0). The camera shots' program pointer and the surface rows past material 121 are masked, as in `tests/releases.rs`.
  - data entries (`dat txt csv mdl mtl ani ani2 cam hmp`) of the archives the match reads: TRAJ, PCDATA/PCCG0, PCANI motions, every PC costume model (skeletons pose the players), courts 0–13 (CMN/GRD01/HOL01/SSN1 HOL01), CMN/GAME, EFFCT, and INPANE's `.ani` entries (the intro banner sets the intro's length, which the RNG follows).
  - Left out: textures (`tm2`, `mti` pixels, `bmp`), sound banks, face morphs/UV, texture animations, costume noise, file names and archive layout. Names are the US ones; JP files answer through `Iso::read`'s aliases (its voice copies of the motion archives digest only their motions).
- `Iso::read_unlogged`, so the digest's reads don't land in the asset-use log.
- `Handshake::sim`, carried in the lobby Hello (`net.rs`, serde-default). `handshake::set_disc(path)` replaces the two `set_pack` calls in `hst`'s `main.rs`. It stamps the pack and works the digest out on a thread (wasm: on the first `here()`). `check` compares the digests when both sides send one. Equal digests meet whatever their pack hashes; different ones are refused, naming both releases. Without a digest on one side, the old pack check applies. A player on a disc image (no pack hash, no check before) is now checked too.
- Tests: `hst-data` `digest::tests` (stable; blanked faces don't change it; a TRAJ table, plant records or a missing file do; texture pixels never counted; JP digests differently from US); `hst-base` `handshake::tests::discs_that_play_alike_meet_whatever_release`.

## Where the releases stand

- US ISO, the old pack and the new (faces blanked) pack: one digest, so they meet.
- Japan: the digest differs (its shot weights, rain odds, missed-serve table and TRAJ tables differ, M163a), so it is refused against a US host. That is right: it would play another game. It can't play at all until M163a1.
- Europe: no digest (`Game::new` refuses the 50 Hz release). It can't be imported anyway (M163a2).
- When M163a2 (b) lands, the regenerated TRAJ tables must go into the digest in place of the disc's EU tables. Otherwise an EU disc digests differently and is refused. Noted on M163a2.

## Not verified / not 1:1

- **No US↔JP/EU rollback/synctest run.** Neither JP nor EU can play a match yet (M163a1, M163a2). The mechanism is tested only on handshakes with synthetic digests, plus the real US/JP digests differing.
- **Which program tables are really sim-relevant was judged from where they're read, not proven.** Doubtful ones (NPC triggers, walkers, cutaway/replay cameras, umpire words lang 0) are included: refusing a peer too often is safer than letting a desync in. A left-out table that does feed the sim would still show up in the in-match desync hash.
- **Digest time on a pack not measured on a slow device.** It reads ~100 MB (mostly the PC costume archives). It runs off-thread on desktop at disc open; on web it runs at the first lobby, synchronously.

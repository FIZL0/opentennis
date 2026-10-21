# C34 — dead code and stale allows (from the 2026-10-21 repo audit)

Follows `../2026-10-21-repo-audit/README-FINAL.md`.

## Done

- `hst-play` `play/snapshot.rs`: whole-file `allow(dead_code)` gone. The one item it was hiding, `state_hash`, has
  only test callers (snapshot, umpire_mod, tick_cost, match_stats/hold tests), so it's `#[cfg(test)]` now.
- `hst-base` `net.rs`: whole-file wasm `allow(dead_code, unused_imports)` gone. The wasm32 check flagged 8 items,
  each now `#[cfg(not(target_arch = "wasm32"))]` like the file's other transport-only items: `AddrFilter` and
  `Signature` imports, `TOPIC`, `BOARD`, `MDNS`, `LAN_WAIT`, `lan_listing`, `Room::take`.
  `cargo check -p hst-base --target wasm32-unknown-unknown` and native `--tests` both clean.
- `audio.rs`: stale `allow(dead_code)` on `SoundBank::wavs` removed (mods.rs uses it).
- Deleted, unused anywhere: texhash `from_pack`, xb `name_hash`, npc `call::PAUSE_OVER`, ai `Guess::target_x` (and
  its now-unused `DOUBLES_SIDELINE` import), hst-oracle's `DONE`/`Hook` re-exports.
- Test-only pub items behind a new hst-sim `testing` feature (`#[cfg(any(test, feature = "testing"))]`), turned on
  for hst-sim's own integration tests by a dev-dependency on itself: `pose::Nodes::from_parts`,
  `shade::receiver_vertex`, and the whole shade texture/receiver pass (`pass_texture`, `receiver_tile` and the
  types and helpers only they use: `PassVertex`, `Wrap`, `PassTexture`, `PassDraw`, `ReceiverVertex`,
  `ReceiverDraw`, `gs_tri`, `pass_*`, `texel_round`, `TARGET`/`SCISSOR`/`LANES`). Cfg on `cfg(test)` alone wouldn't
  reach `tests/`.
- SAFETY comments: web.rs's counting allocator (impl + each `System` call); ios.rs's signal/atexit setup, signal
  handler, getrusage, thermal state, exception handler, memory-warning observer, haptic tick, safe-area read,
  display link; android.rs's signal setup, `OLD`, signal handler, two `JObject::from_raw`s.
- `web/apple-touch-icon*.png` untracked (gitignored; `tools/web.sh` copies them from `icon-180.png`).
- Left alone as the task says: sound.rs's umpire call constants, exe.rs's unused values.

## Not verified / not 1:1

- iOS and Android builds not compiled here: those edits are comments only.
- No gameplay change; nothing to check against the original.

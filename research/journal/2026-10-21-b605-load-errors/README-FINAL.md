# B605 — bad court / sound data errors instead of crashing

From the repo audit (`research/journal/2026-10-21-repo-audit`).

## What changed
- `hst_sim::court::world` returns `Result<World, String>`: every archive read, XB parse, model/material parse,
  placement parse, a missing `.MTL`, a second or missing court collision model, a placed prop with no model and a
  collision face on a node the model doesn't have come back as an error naming the file. Retail behaviour unchanged
  (same world built; the court tests pass).
- Match setup (`play/court_load.rs`, called from `disc` in `play.rs`): on an error it logs it, plays on court 10's
  collision world (the stage-less default) and shows the reason in the match's notice box (`OnlineNotice`) for 6 s.
  Only if court 10's data is unreadable too does it panic (with the reason): there is nothing to collide with.
- `main.rs` stage load: no shade map for a court whose collision world can't be read (logged).
- `sound.rs` `Voice::shout`: the `expect("a shout has keys")` was never bank data — the key pool is never empty
  (only programs 1 and 2 remember a key, and their highest key is ≥ 1). Replaced with a fallback and a comment.
- Court mods don't feed `court::world` (they name a disc layout; the collision is the disc's), so the reachable cause
  is a damaged disc image or pack.
- Test: `play::court_load::tests::unreadable_court_falls_back` (court 99: error text, court 10's world).

## Not verified / not 1:1
- The notice was not looked at on screen (no damaged disc to hand; it uses the existing notice box drawing).
- Falling back to court 10's collision is a remaster choice: the original has no such case. Online, a peer with an
  unreadable court would desync from one without; both peers normally have the same disc.
- Other setup `expect`s on disc data (SYSTEM.CNF, GAME.BIN, PCDATA, intro files) are outside this task's scope.

# T25: `discs::tests::finds_the_other_releases` fails again

## Cause
No commit broke it, so there was nothing to bisect. `fixtures/discs/` (git-ignored, shared) gained
`SCES-54535-proto-0119.iso` (the January 19 European prototype, M163's research disc) on 2026-11-16 at 11:06, after
M166's gate had passed. That disc boots the same `SCES_545.35` with the same `SYSTEM.CNF`. `Edition::of` only
checked the European GAME.BIN for the `MWo3` magic, so the prototype counted as the European release and, sorting
first by name, came back from `beside` in place of the retail disc.

## Fix
`exe.rs` has a European 1.00 layout now (added after M166), so `Edition::of` checks every release's overlay through
`Game::new`: id, size and load address. The prototype's GAME.BIN is the same size but loads at 0x338380, not
0x337a00, so it is passed over, as the module doc says a different version should be. The test also asserts this when
the prototype is in `fixtures/discs/`.

## Not verified / not 1:1
none (disc detection only, no gameplay)

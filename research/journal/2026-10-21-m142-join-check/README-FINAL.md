# M142 — wire the online match check (2026-10-21)

From the repo audit (2026-10-21): `Handshake::check` and `gameplay_hash` were only called by their own tests, and
the `Handshake` resource `load` inserted was never read. Only the build ids were compared at the lobby door.

## What changed

- `handshake::Handshake` is now just what the lobby can't make the same by itself: the build id and the disc pack
  hash. The rules, seats, characters/costumes and the match seed come from the host's `Room` (every peer builds its
  match from it), and a mod's gameplay data comes off the wire (`mods_online::Wire`, already tested to keep its
  gameplay hash), so comparing those again would compare the host's data with itself; the desync hash still
  catches anything that drifts. Those fields, `Handshake::of` and the unused resource are gone; `gameplay_hash`
  stays as a test helper (the mods_online round-trip test uses it).
- `handshake::set_pack` / `pack` hold this app's disc pack hash; `start` (main.rs) stamps it from `Args::iso`.
- `Msg::Hello` carries `pack` (serde default, omitted for an ISO). The host checks a guest's
  `Handshake { build, pack }` against its own (`Handshake::here`) with `check` at the door and on a mid-match
  rejoin, and refuses with its message: the build one is `version_differs` as before; another imported disc reads
  "Refused: your imported disc (…) isn't the same as the host's (…). Re-import an unmodified Hot Shots Tennis (USA)
  disc." An ISO (no pack hash) on either side passes: its disc ID is checked at start.
- Tests: `handshake::tests::same_build_and_disc_join_and_differences_are_refused`; the loopback test
  `another_build_is_refused_at_the_door` now also joins with the same build and another pack and is refused.

## Not verified / not 1:1

- Not tested between two real machines with different packs (the loopback test runs the real host task and
  Hello over iroh). Online lobbies are ours, nothing to match against the original.
- A pack and an ISO of different discs still meet (the pack's hash can't be compared with an ISO); both are
  disc-ID checked at start, so only a modified disc behind a correct ID gets through, and then the desync hash
  shows it.

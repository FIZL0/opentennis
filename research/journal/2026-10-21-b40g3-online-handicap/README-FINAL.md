# B40g3 — online handicaps

Set Handicap now works in online matches (it was local only, since B40g1).

## What changed
- **Lobby data** (`hst-base/src/net.rs`): `Pick` has a `handicap` level (0..=2, left off the wire when it's 0, so older
  rooms still parse). Only the host sets it, with the new `Cmd::Handicap(seat, level)`, in `Room::set_handicap`. That
  bumps the line-up `ver`, so guests have to ready up again. The seat keeps the level when its peer picks again
  (`repick`), when the host picks a computer again (`bot`), when seats are swapped and when the lobby goes vanilla. A
  guest's picks always arrive with no handicap (`take_from` and `repick` clear it), so a guest can't set its own.
- **Handshake / match** (`hst-play/src/play/netplay.rs`): `flags` gives `--handicap` by match slot from the room's
  seats. Every peer builds its match from the same start `Room`, so all bodies run at the same handicap. `Body.handicap`
  is already in the rollback snapshot, and any drift would show in the desync hash. The build/disc `Handshake` is
  unchanged, because the room carries the levels; its doc says so.
- **Menu** (`main_menu/handicap.rs`): in the lobby, the host's L2 / R2 (keys 1 / 3, or Q / E) on any taken seat's card
  steps that player's level down / up. This works for a person's seat or the computer's, including the host's own. The
  legend shows "Handicap" there. `handicap_of` reads the room online. Cards show a level that is set: the full card
  (this machine's players) has the orange pill in its footer, and the compact card (other peers and the computer)
  appends "- 1 weight" to its footer line. Guests see the same.

## Tests
- `net::tests::handicaps_are_the_hosts_and_stay_on_their_seats` — only the host sets levels; they survive repick,
  computer re-pick, swap, vanilla and serde.
- `netplay::tests::flags_carry_the_handicaps` — match-slot order, and every peer gets the same flag.
- `main_menu::handicap::tests::the_lobby_host_sets_each_players_handicap` — L2/R2, legend, open seat, guest.
- `HST_NETPLAY=1 tools/check.sh -p hst-play netplay` passed (36).

## Not verified / not 1:1
- There is no original to compare with: the PS2 game has no online play. Handicap gameplay is B40g1's (run size),
  unchanged.
- I didn't run a real two-machine online match with a handicap set. The flags test and the snapshot already carrying
  `Body.handicap` cover peer agreement.
- I didn't take a screenshot of the lobby cards with a level set. The pill reuses B40g1's layout, which already works
  at any aspect.
- A touch-only host has no tap to step a seat's handicap in the lobby, because the card's hit area takes taps (✕ / move).
  Follow-up added.

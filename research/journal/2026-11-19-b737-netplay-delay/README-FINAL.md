# B737 — per-player Netplay Delay

Settings → Online & Mods → **Netplay Delay**: Auto (2 ticks) / 1 / 2 / 3 / 4 ticks, saved as
`netplay_delay = auto|1..4`. It is this machine's own rollback input delay only; ggrs keeps each peer's delay local
(a peer's pads just arrive that much earlier), so peers need not agree and B731's "same on every peer, from the host's
measured round trip" rule is gone for rollback. Lockstep (`HST_NET_LOCKSTEP`) still uses the host's `Room::delay`,
which it needs to be shared.

- `rollback.rs`: `choose_delay`/`chosen_delay` (a process-wide choice, like Match Speed), `delay_of(choice)`
  (0 → `MAX_DELAY`), `input_delay` now reads the choice instead of the room; `Rollback::with_delay` takes the delay
  explicitly (`new` passes the setting's). The unused `DELAY` constant went (Auto = `MAX_DELAY` = 2).
- `netplay.rs`: re-exports for the menu; test helper `play_rollback_at` gives each peer its own delay;
  `rollback_peers_with_different_delays_stay_in_sync` (host 1 tick, guest 4, 6 ticks of lag each way, 600 rounds):
  same confirmed pads on both, ggrs checksums agree. It replaces B731's `rollback_delay_follows_room`.
- Menu (`main_menu.rs` settings field/parse/save, `options.rs` row, hint, dropdown hints for 1 and 4 ticks):
  `net_delay_row` test. Shots at 16:9 and a tall 7:10 window (context/b737/run2/) lay out fine: the hint panel
  moves under the rows on the tall one.

## Not verified / not 1:1

- Netplay delay is a remaster-only online feature (the original has no online play): nothing to match.
- Not tried between two real machines over a network; the in-process two-peer rollback test with a simulated slow link
  is the evidence that mixed delays stay in sync.

# B602 — netplay: guests can't forge match messages

From the 2026-10-21 repo audit. Before: the host passed every in-match message of a guest on to everyone (only
who's-away and restart held back) and the match never knew who sent what, so a doubles guest could push a state, a
lobby line-up or anyone's pads into the others; the resync/restart piece buffers and the lockstep's waiting ticks
grew without limit.

## What changed

- `Event::Match` carries the peer the message came over: on a guest always `"host"` (the host passes the other
  guests' on), on the host the guest's own peer, and `"host"` for its network's own "a player came back".
- The host's relay (`net.rs`, `from_guest`) drops a guest's who's-away, state, restart and lobby; keeps a guest's
  pads (match and intro) only for the seats that guest holds in the line-up (none left: dropped); rewrites a
  guest's keep-alive / loaded / switched-away to the guest's own peer name, whatever name it put in.
- The match (`netplay::take`) drops those host-only messages (and the rejoin word) unless they came from `"host"`
  (a second line behind the relay, and the guard on the host's own match).
- Lockstep pads: none for this machine's own seats, none more than `2·delay + 60` ticks ahead (no honest peer gets
  past `2·delay − 1`).
- The state's and the restart's pieces stop at 32 MB (`resync::STATE_MAX`): past it the pieces so far are dropped.

## Tests

`net::tests::the_host_drops_a_guests_forged_match_messages`, `netplay::tests::host_only_words_come_from_the_host`,
`pads_only_for_others_seats_and_near_ticks`, `resync_pieces_are_capped`; the netplay match tests (HST_NETPLAY=1),
`a_silent_peer_rejoins_and_a_silent_host_ends_it` and `synctest_through_a_change_of_ends` (ignored ones) pass.

## Not verified / not 1:1

- Not a gameplay port: nothing to check against the original (it has no online play).
- Between a rejoin and the host's restart, the relay's line-up still has the old peer in the seat, so the rejoiner's
  pads are dropped until the restart lands; it doesn't send any before that (it waits for the restart).
- Rollback's datagrams (GGRS) still carry the sender's own inputs as GGRS frames them; their seat ownership is
  GGRS's per-peer handles, not checked here.
- Not tried against a real malicious client, only the unit tests.

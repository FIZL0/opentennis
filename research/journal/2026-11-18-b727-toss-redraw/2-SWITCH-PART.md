# B727 — 2: the spec's open questions answered (2026-11-18)

Spec session (clean-room, no Rust). SPEC.md gains sections 4 and 5; its *Open questions* is now `none`. The port
phase follows 1-SPEC-PART's *Port phase: what to change*, unchanged.

## Found

- **The switch that forces the partial set-up is "this point is a replay"** (SPEC section 4). The game can replay
  a finished point by rewinding to its start snapshot and playing it again. When it does, the serve phase's entry
  restores the score state (the "no rally yet" mark too) and turns the switch on. A live point's entry takes a fresh
  snapshot and turns it off, and so does leaving match-over. It isn't a match mode, and slot 5's live points always
  have it off. Play has no point replays yet, so the port treats the switch as off.
- **The ordering is safe.** The match object and the players get messages at the same level, with the match first,
  so the switch is already set for the current point when the players' set-up reads it.
- **Fault and let** (SPEC section 5). The next-point step clears the last-hitter record before it checks for a
  pending second serve or a let. Faults and lets also go through the point-over phase, so every toss follows the
  clear. This comes from reading the code. No recording covers it yet.

## Not verified / not 1:1

- **The replay switch** isn't checked against a capture. Slot 5's state won't load in the current PCSX2 (see
  1-SPEC-PART), and play doesn't replay points, so nothing in the port reads the switch yet.
- **The last-hitter reset before a fault's second-serve toss** is read from the code only. `rng_s05.bin` ends
  around tick 1464, before any fault. The port's bot-match test will check it once it reaches `match_s05`'s faults.
- **The second replay request path** (the one driven from the replay controls) is described only far enough to
  say that it also sets the switch. Its trigger conditions aren't needed for this task.
- The items from 1-SPEC-PART's list still stand.

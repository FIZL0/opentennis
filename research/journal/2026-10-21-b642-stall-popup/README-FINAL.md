# B642 — the "Waiting for …" stall pop-up flashed up mid-rally

The online notice put up "Waiting for 2P..." once the match had gone 0.25 s (real time) without advancing a tick,
lockstep or rollback (both reset `Netplay::stalled` when a tick runs). Any short hitch on the link flashed it over
the rally.

Change (`play/netplay.rs`): the mid-match threshold `STALL` is now 2.0 s; a shorter stall still holds the match
(the gate simply doesn't advance), silently. The "Waiting for … to load..." line before every peer has loaded keeps
its own 0.25 s (`LOAD_STALL`): there's no rally to interrupt there and quick feedback helps. A peer that goes fully
silent is still handled by `recover.rs` (its own notice and timers, unchanged).

`rollback.rs` needed no change: it only resets the same `stalled` counter when ggrs advances.

## Not verified / not 1:1
- Not tried on a real lossy link; the 2 s figure is the task's "a couple of seconds". Online presentation is ours
  (no original to match).

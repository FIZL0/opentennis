# B732: input delay from the best recent ping

The host picked the online match's input delay from each guest's last single Pong at start, so one slow sample
(a reliable-stream retransmit, or a moment still on the relay) inflated the delay for the whole match.

Now `start` takes each guest's lowest round trip from the lobby's recent queue (the last 5 Pongs, ms, the same queue
the lobby cards already show the minimum of), via `best_rtt`; a guest with no samples counts as 250 ms as before. The
worst of those across guests still sets the delay (`delay_at`).

Test: `net::tests::delay_from_best_rtt` (a 900 ms spike among ~40 ms samples gives the same delay as the low samples
alone; none → 250 ms).

## Not verified / not 1:1

- Not tried in a live two-machine match (unattended run); covered by the unit test only. Online play has no
  original counterpart, so nothing to match 1:1.
- The recent queue holds whole milliseconds, so the round trip fed to the delay loses sub-ms precision (at most one
  tick only exactly on a boundary; immaterial).

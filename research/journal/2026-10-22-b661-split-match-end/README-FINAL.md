# B661 — split view: match-end celebration full screen

## Change
`split_view::step`: once the score says the match is decided (set when the match point's set is won, so from that
point's end: the game-set-match call, the celebration, the result), the split's choice is switched off. `showing`
drops, so the halves' cameras clear and `split` takes its existing "off" path: the second camera is despawned, the
main camera gets the full viewport back, and the sticks stop turning. Because the choice is off, the next match also
starts on the shared camera, the same as every other match. Before this, `split` kept a split that was switched on
through everything and showed the one picture in both halves (`both`), which cut off the celebration in each half.

Test: `split_view::tests::match_end_is_one_picture`: a split 2P singles match is forced into the post-point phase
with the match decided; the split is off, the halves' cameras are cleared and no stick turns.

## Shots
I tried `tools/shot.sh` with `HST_SPLIT2=on` and a 1-game match (`context/b661/run1..4`, montages `m1..m4.png`), but
never reached the last point. The split needs a human on each team, and a `--shot` run can only drive the keyboard
seat. The pad seat is idle, and the match always ended up waiting for that seat (or a keyed P1, see memory) to serve.
I tried singles `k,0` (court 0 drew no court), singles `0,k`, singles `0,k` with `HST_TIEBREAK=1`, and doubles
`-,k,0,-`. All four showed the live split correctly, but none reached match point.

## Not verified / not 1:1
- The `--shot` of a split match's last point: it couldn't be reached unattended (above). Covered by the unit test.
  The full-screen picture is the existing split-off path, the same one Select switching the split off uses.
- Front-end/presentation only, so there's nothing to check against the original.

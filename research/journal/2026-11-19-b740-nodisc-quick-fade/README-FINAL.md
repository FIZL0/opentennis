# B740 — iOS no-disc page: quick fade into the menu

User 2026-11-19: once the ISO turns up, the no-disc page should switch to the menu with a quick fade (a few frames),
not the usual slow zoom/fade, so it's obvious the disc was picked up. Front end: ours to design.

## What changed

- `hst-play/src/mode.rs`: `Mode::quick()`. A quick switch skips the last-frame capture and zoom (`leave`/`zoom`):
  the black loading cover fades in over `QUICK` (0.1 s, real time) over the old mode, holds while the new mode
  starts as usual, then fades out over 0.1 s over the new mode instead of vanishing (its `Loading` component comes
  off at once, so input and `warm_up` behave as after any switch). Other switches are unchanged.
- `hst/src/main.rs`: the iOS no-disc `look` switches to `Mode::new(menu_mode).quick()`.

## Checked on desktop

A temporary `HST_NODISC` mode (like `~/runs/opentennis/nodisc.diff`, not committed): a plain page that switches to
the menu at 3 s, screenshots every 0.05 s from 2.9 s (`HST_KEYS` shot steps, `tools/shot.sh`, 1280×720).
- quick (run1): page up to 3.00 s, black by 3.05 s, black while the menu starts (~0.35 s), menu fading in at
  3.40/3.45 s, full at 3.50 s.
- old path (run2, same page without `.quick()`): page held on screen until ~3.40 s (capture wait + 0.5 s zoom/fade),
  black, hard cut to the menu at ~3.75 s.
So the disc pick-up shows within ~0.05 s instead of ~0.5 s, and the menu is up ~0.25 s sooner.

## Not verified / not 1:1

- Not run on an iPhone: the change to `no_disc_mode` is `cfg(target_os = "ios")` and wasn't built for iOS here
  (one-line call of the new `pub` builder; desktop shows the same `mode.rs` path).
- Not 1:1 with anything: front end is ours to design (PLAN.md), so no comparison with the original.

## Retry (2026-11-19)

First attempt parked only for a full /tmp in an unrelated update test. Re-applied on main (menu_mode now resets
`disc::WAITING` on iOS; no interaction with the switch); `hst-play` mode tests pass, `hst` builds.

# T30 — split screen bench mode

`HST_SPLIT=side|stacked tools/bench.sh LOG SECS` now benches split screen: no `HST_AUTOPLAY`, `--pads k,-,k,-`
(1P team 0, 3P team 1) and `HST_SPLIT2=on|stacked`. `HST_SPLIT2=stacked` (split_view `start_on`) starts split with
the halves stacked, setting `STACKED` since `--play` never applies the menu's Split Screen setting; the start logs
`split view on from the start (side by side|stacked)`, which bench.sh's summary greps.

Check (30 s, `HST_GPU=igpu`, 1280×800, court 1, 5 s reports at 15–25 s): side by side 81.7–84.8 fps, p50 11.7–11.9 ms;
stacked 77.5–78.1 fps, p50 12.6 ms. Shots (`HST_SPLIT2=on|stacked tools/shot.sh … --pads k,-,k,- --shot-at 20`,
scratch in `context/t30/`) show both layouts.

## Not verified / not 1:1

none (bench tooling; split screen is a remaster addition).

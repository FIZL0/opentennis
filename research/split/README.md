# Split screen benches (R12)

- `bench_split.sh LOG SECONDS [hst args]`: `tools/bench.sh` without `HST_AUTOPLAY`, so `--pads k,-,k,-`
  humans stay human and `HST_ENV="HST_SPLIT2=on"` really splits (autoplay makes split never eligible).
- `summarize.py LOG…`: one line per log, the steady-state average from 15 s on: fps, 1 % low, CPU ms, game ms and
  the main passes' GPU ms.

Findings: `research/journal/2026-11-21-r12-split-screen-cost/`.

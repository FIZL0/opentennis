# P140 — Bevy scheduler wake-ups

Goal: try fewer compute threads or the single-threaded executor to stop the compute pool burning CPU on waking and
parking its threads for tiny systems.

## What landed

`crates/hst-base/src/sched.rs`, called from `main.rs`:

- `TaskPoolPlugin` with the compute pool capped at **4 threads** (default). `HST_COMPUTE=N` overrides it
  (`HST_COMPUTE=99` brings back Bevy's split: on this 16-thread desktop that's 4 I/O + 4 async + 8 compute).
- `HST_EXECUTOR=single` (main-world schedules on the single-threaded executor) / `single-all` (render world too) /
  `multi` are kept as measuring knobs. The default stays Bevy's multi-threaded executor. Schedules a mode adds later
  are switched by a `Last` system.

Phones, the Steam Deck and other 8-thread devices already got 4 compute threads from Bevy (8 − 2 I/O − 2 async), so
nothing changes for them. 2-core machines get 1 either way.

## Measurements

`tools/bench.sh` (4-bot doubles, 1280×800, uncapped ≈ 240 fps on this monitor). "cpu" is the HST_FPS log's process CPU
ms/frame, averaged over the run (first 10–15 s skipped). Load average is from the other agents sharing the
machine (16 threads); it moved a lot, so the variants were run interleaved, over several rounds.

| round (load) | default (8) | c4 | c3 | c2 | single | single-all | single+c2 | single+c4 |
|---|---|---|---|---|---|---|---|---|
| 2 (22–50) | 8.61 | 3.69 | – | 4.20 | 6.37 | 8.70 | 3.84 | – |
| 3 (27–39) | 9.00 | 4.26 | 3.62 | 3.58 | – | – | – | 3.79 |
| 4 (8–24) | 4.03 | 2.70 | 4.09 | 2.41 | – | – | – | 4.12 |
| 5, final build (28–38) | 8.70 | 5.13 / 3.86 (new default) | | | | | | |

1 % lows in the last 5 s window: default 49–126 fps, c4 79–237 fps. On a busy machine the 8-thread pool was
the worst variant every time, because its wake-ups compete with everything else. Neither single-threaded executor
beat a capped pool. Alone, it still leaves the 8-thread pool for render and transform propagation, so it saved less.
c2 and c3 had worse 1 % lows than c4 in some rounds, so 4 is the default.

Low end: `HST_SLOW=1` (tools/slow.sh's 2 cores at 120 %): new 2.52 / 2.48 ms, old 1.98 / 2.44 ms CPU per frame. The 1 %
lows swung both ways between runs (noise). Both settings build the same pools there.

Logs: `context/p140/run2`…`run6`.

## Not verified / not 1:1

- No samply profile: samply isn't installed here. Total CPU came from the HST_FPS log (`/proc/self/stat`) instead,
  so I didn't confirm which call stacks the saved time came from.
- Not run on a real phone or handheld: no device is attached. The argument for "no worse there" is that Bevy already
  gave those 8-thread devices 4 compute threads, so their pools are identical. The 2-core desktop limit confirmed
  "no change" for small machines.
- The P139 serial transform fix hasn't landed on this branch, so these numbers include the parallel transform spin,
  which the smaller pool also shrinks. After P139 the gap between 8 and 4 threads should be smaller, but still in
  favour of 4.
- Gameplay is untouched (scheduling only; the sim is deterministic per tick and runs inside a system either way).

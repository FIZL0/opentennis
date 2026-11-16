# P138 — Phone CPU: where the other ~4 ms go

## No iPhone trace
The user can't attach Instruments to the iPhone (Xcode/iOS version mismatch, see the earlier block), and none came
with the reopen. So the suspects were measured on desktop instead, with the phone log's settings (High: 100 % scale,
shadows high, 4x MSAA, `--fps 120`), a 4-bot doubles match on court 1: samply (`--unstable-presymbolicate`, seconds
15–48 of a 50 s run) plus `tools/profile.sh` (Bevy spans). Captures are in `context/p138/` (scratch).

Desktop, 4 mod characters (other-game mods, CPU face morphs forced on with `HST_CPU_MORPH=1` as on iOS), per 120 fps frame:

| where | ms/frame |
|---|---|
| main thread (game schedules + extract) | ~1.7 |
| render thread (prepare/queue/batching ~0.9 + encode/submit 1.1, of it shadow pass 0.22) | ~2.0 |
| Bevy compute pool (wake-ups, queue pops, futex) | ~0.6 |
| audio, input, rest | ~0.1 |
| total | ~4.9 (disc characters: ~3.8) |

## Verdict on the suspects
- **Bevy worker threads spinning**: yes, the biggest. P139 (landed) removed the transform-propagation spin, ~28 % of
  all CPU in its desktop profile, which matches most of the phone's unexplained ~4 ms. The pool's remaining
  ~0.6 ms of wake-ups is P140 (queued).
- **CPU face morphs of mod characters**: no. `cpu_morph::draw` is ~0.01 ms/frame, the vertex writes ~0.03.
  Next to them, though, `character::uv_offsets` asked `cpu_morph::shared` (mutex + SipHash map lookup + a linear
  search over the parts) for every drawn part of every rig, every frame: 8 ms/s with mods vs 2 ms/s with disc
  characters. **Fixed**: parts look themselves up first, and only meshes that aren't parts go to the registry,
  which now uses Bevy's foldhash map. After the fix: `uv_offsets` 3.2 ms/s, `cpu_morph::shared` 0.5 ms/s, on a
  run where the machine was ~40 % busier overall (other agents), so the real drop is larger.
- **Render extraction / shadow casters**: real but ordinary. Extract is ~0.34 ms/frame with mods (0.18 with disc
  characters: more parts and skin joints, `extract_skins` 4x). The shadow pass encode is ~0.22 ms. Nothing wasteful
  stood out, so no change.
- Also seen: with mods, costume sway (`noise::draw`) is ~25 ms/s, mostly ps2-float's VU ops called out of line
  (no `#[inline]`, no LTO). Follow-up **P141**.

Sim untouched: the change is in drawing (UV offsets of parts, the same index for every mesh as before).

## Not verified / not 1:1
- No iPhone measurement: the user can't attach Instruments. Desktop proportions stand in for the phone. An iPhone
  log (HST_FPS) after P139 + this would confirm it.
- The desktop runs were noisy because other agents shared the CPU. The comparisons are within a run (per-function
  share), not absolute.

## Wrap-up
The work is done (hst-base tests pass), but `tools/done.sh`'s full suite failed for reasons outside this task:
`discs::tests::finds_the_other_releases` (a prototype EU ISO another task put into the shared `fixtures/discs/` at
11:06, filed as B712), and once `net::tests::a_crossplay_guest_rejoins_on_the_hosts_data` (flaky in the full suite,
passes alone 3/3, filed as B711). So it is committed as blocked: tick it once B712 lands and done.sh passes.

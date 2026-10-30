# B700 port: CPU serve press countdown

Implemented from `SPEC.md` with the decompile locked.

- `hst_sim::serve::AI_LOOK` (2..=180): the AI's serve contact pick (`ai_search`) skips the next two frames and looks
  180 out (a bounce in the skipped frames still ends the search). The play side predicts 182 path points for it
  (frame 180 needs the next one for its speed).
- `hst_sim::serve::AiPress`: the wait / take at ≤ 8 / count down from the next frame / press at `left + err ≤ 8` rule.
  It replaces the play side's old `bot_due` frame (`8 − err` clamped at 0, which let a negative error press early).
  The `Serving.bot_due` field keeps its name (still the "a CPU serves" marker for `pads_aim`), now holding the
  countdown; it is in the snapshot (netplay tests run, 37 pass).
- The offset isn't forced: it comes out of the existing swing search.

## Tests
- `hst-sim/tests/serve.rs` `cpu_serve_presses_at_most_eight_out`: err −4..4 against a pick walking 12..0: press at
  `min(8, 8 − err)`, pick asked only until it is 8 out.
- `play::serve_retime::retimed_cpu_serve_keeps_its_kind` now asserts `offset == −max(err, 0)` for err −4, 0, 2, 4 and
  the server's own drawn error, on every strong-toss CPU serve until a flat and a topspin are both seen (all-CPU
  singles, Will vs row 6). Passes; kind/stick/AI draws still unchanged across errors.
- `ai_serve` fixture tests (the pick against the original) still pass; the replay tests run in done.sh.

## Not verified / not 1:1
- Doubles CPU serves: same code path (`bot_serve`), but not recorded live in doubles (as the spec notes).
- No new PINE recording in the port phase: the spec's live evidence (19 + 28 serves) is what the table and the
  tests are built from; the special-serve share (~70 % for Will) was not measured over a long match.

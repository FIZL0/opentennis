# B700a port: a CPU serve's kind and flat-or-not don't follow the press frame (FINAL)

Port phase of `SPEC.md`. Nothing to change in the game: the port already behaves as the spec says. Added the test
the spec asks for, which pins it down.

## Test

`crates/hst-play/src/play/serve_retime.rs`, `retimed_cpu_serve_keeps_its_kind` (`tools/check.sh -p hst-play
retimed_cpu_serve`, ~1.4 s). All-CPU singles (chars 11 v 6, court 0). At every strong-toss CPU serve it snapshots the
match at the toss (`snapshot::save`) and replays the serve with the server's serve timing error forced to −4, 0, 2
and 4 (`ai_timing.serve` and the press due). In all four it asserts the same:

- swing kind drawn at the press (the AI's serve plan and the press kind);
- contact tick counted from the toss (the same ball frame whatever the press);
- contact stick (`bot_aim`) and the singles dash;
- shot kind and class as struck (flat included);
- the AI generator's next value once the aim is drawn.

It also asserts the offset is `−err` for err ≥ 0, and that the AI generator moves on exactly three ticks from the toss:
the press, the tick before the contact (the aim), and the contact. It runs until it has seen ≥ 6 serves with at least
one flat and one topspin. Run: 6 serves, one flat (stick (0.008, 0.844) toward the net), the rest topspin or slice.
All four timings gave the same serve.

## Draw order (spec *What the port should check* 2)

- **Swing-kind draw at any press frame:** between the toss and the press nothing draws on the AI generator in the
  port (press at 12, 8, 6 or 4 frames out gave the same kind and plan). Holds.
- **Aim tick:** the port draws the aim on the tick before the contact (`left` 2 → 1). The original draws it on the
  contact frame, ahead of the contact. The test shows no other AI draw falls between the aim and the contact's own
  draws. The aim reads nothing that moves in that tick (serve level, swing, court side, hand, end), so the one-tick
  label gives the original's sequence: kind, aim, contact. Changed nothing. No existing app-level recording checks
  the AI generator across a CPU serve. The P11n check was a 60 s run, not bit-exact. This test is the check now.
- `stick_kind` and the stick the contact reads are unchanged (spec 3).

## Not verified / not 1:1

- **err < 0 offset:** the port still presses early (offset +4 at err −4). The spec's `−max(err, 0)` on that side is
  B700's press rule, which is not ported yet. The test asserts the offset only for err ≥ 0. B700 has a `ctx.py`
  note to extend it.
- **Pad byte rounding:** the port's contact reads the float aim stick, not the pad's ×127 bytes. The spec says that
  gives the same direction-test answer. Not checked at the exact 60° edge.
- **Doubles CPU serves:** not covered by the test. The spec measured singles only.
- **Not compared against a PINE recording:** the test checks the port against itself across timings, which is the
  spec's invariant. The original's side is the research session's live intervention (`b700a_err_poke.py`).

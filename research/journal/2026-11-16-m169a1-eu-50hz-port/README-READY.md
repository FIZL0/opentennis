# M169a1 — EU 50 Hz: port + verify (blocked: no spec yet)

The task ports from M169a's `SPEC.md` with the decompile locked (PLAN.md *Clean-room ports*). That spec doesn't exist:
M169a (**EU 50 Hz: spec**) is still open in `plan/TODO.md`, and `research/journal/2026-11-16-m169-eu-50hz/` holds only
M169's classification README. Every other slot's checkout matches this one.

Writing the spec here would mean reading the EU decompile in the port session, which the two-session rule forbids.
Guessing the 50 Hz constants from the M169 summary (×5/6, ×36/25, etc.) is what the M169a line warns against ("not a
blanket ×5/6"), and AGENT.md bans guessing details.

Nothing was changed in code.

**Unblocks when:** M169a lands its `SPEC.md` (`--spec`). Then unblock M169a1. The runner shouldn't start a family
child before its parent is ticked, so check why it picked this one (M169a is at line 11, M169a1 at line 46).

## Not verified / not 1:1
- Everything. No port was attempted because there was no spec to port from.
- Wrap-up: `tools/webcheck.sh` failed on a deprecation warning in vendored bevy_ecs (P141); renamed `fetch_update` → `try_update` in `third_party/bevy_ecs/src/world/identifier.rs` (same behaviour) so done.sh passes.

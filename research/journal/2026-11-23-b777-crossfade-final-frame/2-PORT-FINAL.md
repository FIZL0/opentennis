# B777 port: the crossfade's last frame and the switch-tick bind

Status: FINAL. Port of the spec in `SPEC.md`. `README-PART.md` and `SPEC.md` are unchanged.

## What changed

- `hst_sim::motion::Fade` gained `mixed`: the outgoing motion this frame's unheld mix samples. `Fade::tick` sets it
  before the countdown drops the outgoing motion, so the fade's last frame (new clip time 7, weight 0) still mixes.
  The fade's own fields still drop the outgoing motion on that frame as before. The anim_s05 fade-state test is
  unchanged and passes, so a new motion started after the fade is handled as before. The held fade is unchanged;
  the spec scopes it out.
- The stored pose (`bodyhit::pose`) and the doubles stand test's same-tick read (`doubles_ai/scratch.rs`) mix with
  `mixed` for the unheld fade.
- The stand test's read binds its working copy of player 1's pose when player 1's motion command starts a different
  motion this tick (rest rows, single-key tracks at their key). The kept state is not changed.

## Verification

Temporary `HST_B773` print of player 2's read, reverted. B773's fit scripts. Logs in `context/b777/run1/`.

- US bot match (`ai_net_s05.bin`, 2648 entries): 708 misses. B773 baseline was 1765; the spec's hack run was 717,
  and the difference is B775's start-state change landing since. Ticks 888 (0.4028), 237, 243, 228 and 250 now match
  at shift 0.
- EU bot match, rally window 38056–39692 (589 entries): 12 misses, down from 282. These are the same 12 the spec
  predicted: ticks 7–14 (B779's in-fade residual) and 69, 188, 392, 547 (trail colour, B776).
- `the_us_bot_match_plays_as_recorded` and `the_european_bot_match_plays_as_recorded` pass.
- `tools/check.sh -p hst-play fifty_hz` (136 tests) and `-p hst-sim --test motion` (16 tests) pass.

## Not verified / not 1:1

- The drawn pose (`character::animate` in `hst-base`) was not changed. It samples each frame from the clips' keys
  rather than keeping leftover node rows, so the stale 1/8 blend the spec describes does not arise there in the same
  way. Not checked against a capture; rendering only has to look right.
- The held fade's last frame: left as play had it, since the spec scopes it out. No probe exercises it.
- EU ticks 7–14 (in-fade residual): B779.
- EU trail-colour ticks 69, 188, 392, 547: B776.
- US remaining misses: other scratch writers and the idle clock offset (B775's), as listed in `README-PART.md`.

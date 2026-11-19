# B735: rollback corrections glide on screen

## What
`crates/hst-play/src/play/smooth.rs` (new): a draw-only `Smooth` resource holding offsets (game space) for the
drawn ball, each player and the red landing marker. `rollback::handle` notes what the match had at tick F when ggrs
asks for a load (`Seen::of`), and once the re-runs are back at F compares (`smooth::correct`): what moved is held
where it was drawn (offset += before − after) and shrinks linearly to nothing over 5 frames (`fade`, 5/60 s of
frame time, restarted by each new correction).

- Never smoothed: this machine's own seats (`Rollback::local_seats`), so controls stay instant.
- Snaps (offset dropped, drawn at the corrected place at once): the ball beyond 3 m, a player or the red marker
  beyond 2 m, a hit one run had and the other didn't (shot count differs, or the ball's ground direction reversed),
  the ball in play in one run and not the other (point over / next point set up). The red marker appearing or
  going snaps too.
- Applied in Update: `smooth::shift` after `draw` (figures, ball model), `ball_shadow::place` adds the ball offset
  (shadow + outline follow), `smooth::marker` after `effects::draw_marks` (`LandingMarks::red_root`, new).
- Not in any snapshot or hash, never read by a tick: sim, RNG and state hashes untouched (rollback/synctest tests
  pass unchanged).

A first try snapped on any phase change; the serve's Serve→Rally hand-over then snapped 0.8 m corrections at the
toss, so the rule became "in play vs not".

## Measure (context/iphone-desync/exp_ball_corrections.diff's drawn-vs-true test, now
`drawn_ball_corrections_at_9_ticks`, ignored; 9 ticks each way, 9000 frames, default input delay 2)

| machine | | frames off truth | warps | frames jumping > 25 cm past the true move | snaps (≥ 3 m) |
|---|---|---|---|---|---|
| host | before | 126 | 20 | 34 | 10 |
| host | smoothed | 205 | 20 | 24 | 10 |
| guest | before | 397 | 59 | 84 | 18 |
| guest | smoothed | 639 | 55 | 53 | 18 |

Visible jumps drop by about a third. Frames drawn off the truth go up, as they should: the glide is the drawn ball
taking 5 frames to reach the corrected place instead of being there at once. The worst jump (2.0 / 1.85 m) is the
same both ways: debug logging showed it isn't a correction. It's the true path's own jump (the serve putting the
ball in the server's hand) seen one tick late because the harness's frame lands a tick behind. Other remaining
jumps are the forward guess itself going wrong (drawn on a guessed press before any rollback), which this can't
smooth. `drawn_ball_corrections_glide` (900 frames, runs with the netplay tests) asserts smoothed ≤ before and that
neither machine's own players are ever moved.

## Not verified / not 1:1
- Not seen live over a real connection (unattended run; a headless harness measures it). The feel of the 5-frame
  glide and the snap distances (3 m / 2 m) are my choice, not tuned by eye.
- Big jumps snap rather than cross-fade (the task allows either).
- The ball's trail ribbon, the bounce/hit effects, name tags and anything else placed from a player's or the ball's
  match position rather than the figure/ball transforms aren't offset: during a glide the ribbon's head can sit a
  few cm to ~1 m off the drawn ball for up to 5 frames.
- The camera still follows the match's ball (not smoothed).
- Not 1:1 with the original: online play is ours, the original has none.

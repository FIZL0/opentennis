# B650 — split screen keeps the alternate cameras through the call

**Cause.** The split view (`play/split_view.rs`) only gave each half its own camera (the alternate camera behind that
team's human) in the serve and the rally. A fault, let or point puts the match in the post-point phase, so each half
fell back to the shared match camera, still split. The one-human alternate camera (`alt_camera.rs`) already
stays on through that phase and only gives way to cut-aways and replays.

**Fix.** `plain_play` now also accepts the post-point phase. Cut-aways, instant replays, the intro and the change of
ends still show the one picture in both halves, as before. Like `alt_camera`'s (B628), each half's shot now looks at
the ball where the rally left it once the point is over, so it doesn't follow the dead ball around during the call.

**Test.** `split_view::tests::halves_stay_through_the_call` forces the post-point phase on a 2P split match.
It checks that the halves stay on their own cameras, that the shot's ball is held, and that the change of ends goes
back to the one picture.

## Not verified / not 1:1
- I didn't look at it on screen. The check is headless, on the condition and the held ball, and there was no `--shot` or
  live run. The split is the remaster's own feature (it isn't in the original), so there's nothing to compare against.
- The test sets the post-point phase directly instead of playing a real fault, to keep it short.

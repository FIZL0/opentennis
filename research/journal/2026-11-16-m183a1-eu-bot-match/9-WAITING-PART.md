# M183a1 part 9: after B766, B767 and R14 (PART: blocked on B768 and B769)

Reopened because B766, B767 and R14 landed. Re-ran both whole-match probes (2026-11-23):
`cargo test -p hst-play --lib <name> -- --ignored --nocapture`.

## US (`the_us_bot_match_plays_as_recorded`)
- 27 of 112 phases match (12 before B766). B766 fixed the serve set-up tick at 30–40 and at deuce. B767 made game 2's
  first serve a fault, as recorded.
- Players part at recorded tick 6899. Game 2's first-serve fault point-over lasts 80 ticks in the game and 38 in
  play, and the AI draws around that serve's press and contact differ. That is B769 (open).

## EU (`the_european_bot_match_plays_as_recorded`)
- Unchanged: 3 of 144 phases match.
- Point 2's serve set-up runs 164 ticks in the game and 154 in play.
- R14 found that nothing in the game's set-up waits. The "5 ticks late" AI reset was 5 stalled vsyncs (2 at phase-0
  tick 124, 3 at tick 250) that the vsync-indexed generator test counts as ticks. The match probe already counts in
  game ticks (the phase tick counter), so the 10-tick set-up gap is real.
- Next is B768 (open, filed by R14): map the generator test's vsyncs to ticks; check that play's new-point AI reset
  lands in the phase-change tick (play spreads its 89 draws over three ticks, the game does them in one); then follow
  the AI generator into point 2's set-up, where part 6 saw the server toss while the game's server is still bouncing
  the ball.

## No code changed this session.

## Status
Blocked on B768 (EU point 2's set-up) and B769 (US game 2's fault point-over). When they land: EU tick for tick, then
flip `match_speed::FIFTY_HZ_SIM` and add the release note.

## Not verified / not 1:1
- Whole-match proof is still not done in either build: US parts at tick 6899, EU at point 2's set-up.
- `FIFTY_HZ_SIM` is not flipped and there is no release note.
- The earlier parts' items stand.

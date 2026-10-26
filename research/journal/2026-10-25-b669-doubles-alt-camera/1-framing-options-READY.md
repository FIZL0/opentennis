# B669 round 1: doubles zoom framing options (concept/doubles-alt-camera)

On `concept/doubles-alt-camera` (72b2bfcc): `HST_ALT_FRAME=base|pin|dolly|crane` reworks the doubles zoom's view
(`aim_team`'s output, before the 5 % easing) and forces the alternate camera on. Unset: identical to main (alt_camera tests pass).

- `base`: main's zoom. The eye backs away along the ground, so the bottom ~40 % of the screen is empty ground.
- `pin`: same eye, tilted up until the nearest human's feet sit 85 % of the way down from the middle. Fills the bottom; lots of sky/backdrop on top.
- `dolly`: moves in along the ground until the nearest feet reach that spot, then widens the lens (half-angle at most 0.6) so both humans still fit across.
- `crane`: lifts the eye 0.35 m per metre it stands more than 6 m from the nearest human, then pitches down onto their feet. Most of the court, little sky.

Test case: `--play --court 1 --pads 0,k,-,-` (1P idle at the serve, 2P on the keyboard), `HST_KEYS="1:KeyA*3.5"`
runs 2P to the far sideline at the net, shot at 5 s, `HST_WEATHER_SEED=1234`. Shots: `context/b669/{base,pin,dolly,crane}-far.png`, `grid.png`.

Try it: `git switch concept/doubles-alt-camera`, `HST_ALT_FRAME=crane cargo run --release -p hst`, doubles with two humans on one team.

Next: the user picks one (or a mix); then tune it and check rallies/lobs (the ball must stay in the picture) and the split halves (`width` < 1).

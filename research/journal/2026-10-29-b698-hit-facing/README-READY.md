# B698 — a hit player faces the ball before the knock-down

## Finding: the port already does this

The hit handler (`play/bodyhit.rs`'s `struck`, since P25a) turns the hit player against the ball's last move (flat,
normalised) on the hit tick. That is the way the ball came from. It sets the sim facing and the drawn yaw together,
and starts the hit-by-the-ball motion on the same tick. The stock hit clip (`re_ball`, 0x2b) falls backwards: on
every stock character its head drops from ~1.1 m to ~0.1 m along the clip's local −forward. So the head always ends
up along the ball's way, away from where the ball came from. Nothing turns the player afterwards: locomotion, presses,
the reaction (`react` keeps 0x2b) and the draw all leave the facing alone. A per-15-tick log through the post-point
showed the facing held until the next serve.

Checked live with the remaster (temporary `HST_B698=<player>,<deg>` hook, not committed; release `--shot` clips under
`context/b698/`):

- Ball from the player's right mid-rally (`stage 1`, bot doubles): the player turns right and falls to the left.
- Ball from the net side (the usual case): the player faces the net and falls towards the camera.
- A human P2 (`--pads -,k,-,-`), ball from the net side: same, and the facing holds through the post-point.
- A mod character (Fore! Phoebe in slot 0): mods have no hit clip of their own (their `motions.glb` only carries
  `re_gu`/`re_di`/`*_set`), so they play the donor's clip. They fall away from the ball the same way.
- The instant replay re-simulates the point (snapshot + pads), so it repeats the same turn.

Added `play::bodyhit::tests::a_hit_player_falls_away_from_the_ball`. For six directions it calls the hit handler,
then checks two things: the drawn facing equals the sim facing, and the hit clip's head travels along the ball's way
(cos > 0.9) from first to last frame. When I flipped that check, the test failed with the head moving 0.37 m along
the ball's way.

No code change, so no release note: nothing a player would notice changed.

## Needs the user

Every case I tried already turns and falls away from the ball, so I couldn't reproduce what was seen. To pick this up
again, I need to know where it happened: which court and characters, singles or doubles, human or CPU, whether in an
instant replay or an online match, and which way the ball came from. A save or `--shot` setup would be ideal.

## Not verified / not 1:1

- The case the user saw: not reproduced (see above).
- I didn't compare against PCSX2 this round. The facing rule was already proven against the original in P25a, and
  this task is presentation only.

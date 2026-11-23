# 14: the European bot match, tick for tick

All 144 phases match, and every player and the ball stand where the recording has them on every tick of 31000
(`the_european_bot_match_plays_as_recorded`, `off_runs`), but for the items below. `FIFTY_HZ_SIM` is on.

Found from the recording (ticks are the recording's):
- The serve's stance is recorded at the toss, not again at the strike.
- A smash's body step ramps over every frame to contact, not the last 8 (5202, 25736, 30743). B722's assumption was wrong.
- A player hit by the ball is carried by the knock-back's root path, its sideways part dropped.
- A reaction's root step goes through the mover with the partner. A team reaction without a path is still pushed
  (20091); a solo one is not (24982).
- A dive's lunge ends when the partner pushes the diver past the slide (3123, 10443, 13349).
- A creature strike ends the computer players' path a tick later than it was (7165, 11943, 20077, 25833, 29377).
- The rally check reads the ball before its step after strokes (27401); a dive ends on its own tick (B720's "positions
  don't change" in the 5-tick hold was wrong); a lob blurs the farther opponent first (30670; US 9588).

## Not verified / not 1:1
- 7175-7280, player 0: a reaction starts mid-turn, and the game's player faces one 22.5 degree turn further than ours, so
  their celebration walks another way until the next set-up. Tried: one more stand step on the reaction's tick. It
  fixes this and breaks the same case at 11953 (player 2) and the lunging ones, so the rule is still unknown.
- The held ball at serve set-ups (about 7 ticks each) follows our hand's pose: presentation.
- Runs of three ticks or fewer are ignored (our set-up places everyone a tick before the recording shows it).
- `paths`' knock-back path is proven by this recording only. One recorded match, doubles, all computer players;
  singles, humans and other courts at 50 Hz are not recorded.
- The earlier parts' items stand.

# B774 spec: the European net player's press at point 3's tick 1588

The European bot match (M183a1), point 3's rally, recorded tick 1588: the near-side computer net player (player index 2)
presses. The game moves it on the press tick and play does not, so the two players part at 1588.

## What the game does

1. On ticks 1585 to 1587 the player runs from its after-bounce chase spot (see B772), about 0.078 per tick, direction
   (0.47, −0.88).
2. On tick 1588 the computer player's routine presses. The AI judged the contact as a dive, so the routine's own stick
   move for the tick is dropped. The press is not a dive start: it finds the ball on the player's side and plans an
   approach run toward the press point, direction (0.9974, 0.0714), 19 frames counting the press tick.
3. The game moves the player on the press tick along the approach direction, by 0.0790. This is an ordinary run step
   (the same run the stick drives: full speed growing over the character's agility frames, turning toward the new
   direction); its growth count carries on from the run the player was already in, not restarted, so the steps keep
   growing: 0.0796, 0.0801, and about 0.0006 more per tick. Play's existing run step gives these exact values.
   The run is still going on tick 1606, the 19th frame.
4. On the search tick, 1607, the approach is over and the player does not move.
5. On ticks 1608 and 1609 the player moves 0.068 on each, then stands at x −0.8966.

## Rule

When a computer player's routine presses and the press plans an approach run:

- the run's first step happens on the press tick, along the approach direction, in place of the routine's own stick move
  for that tick. This holds whatever the stick says, including when the AI judged the contact as a dive and let go of
  the stick;
- the step is the ordinary run step, its growth count carrying on from the run the player was already in;
- the approach then runs its remaining frames;
- on the search frame that follows, the player does not move.

With no approach planned, the press tick is unchanged from today's port: on a dive-judged press the player takes no
step that tick and its body is not stopped either (it keeps the run it had, so the dive search the next frame sees a
running player, B720); on any other press the routine's own stick move runs as now.
A press while a stroke is already pending changes nothing.

The rule is the same in both builds. Nothing in it is specific to 50 Hz.

## Evidence

- The positions the game gives player 2 from 1588 to 1609 are exactly the ones this rule produces, to the last digit
  printed in the recording.
- A live capture from an earlier task (B132) showed the rule for human presses: an approach planned for N frames runs
  exactly N frames, the press frame included, and the next frame the player does not move. The game's computer player
  goes through the same press handling, through its pad, so the same rule holds for it. Here the recording shows 19
  moves counted from the press tick, and no move on the search tick.
- What this recording proves directly is the dive-judged press with an approach planned (its first step on the press
  tick, none on the search tick). The non-dive press with an approach planned follows from the same press handling
  (B132's human presses, which take no stick move once an approach is planned); no recording yet shows a computer
  player's non-dive press whose stick differs from its approach, and a trial with the rule applied to both kept the US
  bot match and every play test passing.
- Play's human path and its singles and fallback bot paths already follow this rule. The doubles computer routine does
  not, which is the whole gap.

## For the port

Acceptance:

- The European bot match from tick 1302 (the probe): player 2 matches the recording past 1588, to at least 1885 (its part
  at 1886 or later).
- The US bot match is unchanged.
- B132's approach test and the bot dive test still pass.

## Open questions

none

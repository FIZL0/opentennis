# P119 spec: the smash-whiff test's path horizon and middle height

What the original does when a press finds nothing to hit (its own ball, no stroke, volley, smash or dive in reach)
and it has to choose between a smash at nothing and a ground stroke at nothing. In the port that is play.rs `whiff`
(with `miss` set) and `swing::smash_whiff`. Everything outside the two points below stays as it is; the recorded
whiffs (`tests/motion.rs::recorded_whiffs`) already match.

## A. How many path frames the test looks at

The test walks the same predicted ball path the press's contact search was given. That path is built once per press:

1. Start from the **hitter's own** search horizon: the number of frames in its own timing-grade table, the count
   its own contact search uses (`g.reaches[i].grades.len()` in the port, the same value `find_contact` passes).
   Not character 0's.
2. If that count is below **16**, raise it to 16. (A grade table is never shorter than 8 frames; retail tables
   are well over 16, character 0's is 28, so this only matters for a mod.)
3. Limit it to the ball samples the prediction still has (the same limit the contact search's path has; the
   port's `predicted_path` builds as many as asked, as for the search).

The smash/volley/ground search itself only reads the first *own count* frames of that path; the 16-frame floor
only makes the whiff test (and nothing else in that search) see further. The dive keeps its own 16-frame path.

## B. The height the path must rise above

The comparison height is the **hitter's own** TParam column 64 *middle* cell (the second of the three
top/middle/bottom smash-window values), read like the other two: the integer centimetres ÷ 100 as f32 (the port
already parses it as `ReachStats::smash[1]`). It is a value read from the table, not worked out from top and bottom.

It differs from the port's current `(top + bottom) / 2` even on retail data: with the port's parse (chopped ÷ 100)
and its native f32 midpoint, characters 7 (305/265/225 cm) and 9 and 10 (280/240/200) come out one ulp above
the middle cell (2.65 vs 2.6499999, 2.4 vs 2.3999999). For the others they are equal. A mod's middle cell can be
anywhere; the game uses it as given.

## C. The test, unchanged, for reference

Walk the path from frame 0 in order and stop at the first frame whose bounce count is above 1 (the ball has
bounced twice). At each frame before that: if the frame's ball height is **strictly above** the middle height (B)
**and** the ball as it is now (this frame's ball, the same for every path frame) is less than 2 m across the ground
from the player (x and z distance, squared sum < 4), it is a smash at nothing. If no frame passes, it is the ground
stroke at nothing on the side the ball passes, as now.

## For the port phase

- The whiff needs the hitter's own grade count (with the floor) and own middle cell; `Reach` has no middle cell
  today. Character 0's base `Reach` and every player's `reach_of` would each need it; a mod character's comes from
  its own TParam row like the rest.
- A test with a reach whose grade count differs from 28 and whose middle cell is not the midpoint: a path that
  rises above the middle only after frame 28 (or only between middle and the midpoint) must flip the choice.
  Replays and `recorded_whiffs` stay as they are (for characters 7, 9 and 10 the choice changes only on a sample at
  exactly that one-ulp height).

## Open questions

none

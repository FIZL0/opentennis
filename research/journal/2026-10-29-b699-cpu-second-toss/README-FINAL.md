# B699 — CPU second serves keep their strong-toss chance

Cause (found in `2026-10-29-will-curve-serve`): each character's strong-toss chance on a second serve is read off
the disc when the player loads (`load_player`), but the point placement (`reset_positions`) rebuilds every player
from `Player::default()` and carried only the AI row, `ai_hit`, the mind and the formation. Setup runs that placement
right after loading, so `ai_second` was 0 from the first point on. `AiParams::serve_toss` then tossed weak on every
second serve (or underhand at serve level 0), whatever the character.

Fix: `reset_positions` carries `ai_second` along with the other AI state.

Test: `play::ai_setup::tests::second_toss_survives_the_placements` (singles, chars 6 and 1, all CPU): after setup
and after another placement each player's chance equals the disc table's (30 and 0). Fails without the fix
(`[0, 0]`).

With real second-serve tosses the all-CPU singles match in
`doubles_ai::serve_over::a_singles_receiver_enters_its_rally_in_the_same_frame` takes another course: in hand-over 1
the point ends in the same tick the receiver goes over to its rally (its net rate moved 60 → 65 by the point-result
message), so that tick also holds the point-over draws and the test's two-roll model doesn't fit. The test now skips
hand-overs where the receiver heard a point-over message in that tick; the rest still pass the order check.

## Not verified / not 1:1
- Not re-recorded against the original over PINE: the toss rule itself (`serve_toss`) was already ported and its
  chance table comes from the game program; this only stops the remaster from wiping the input. The will-curve
  journal's comparison is the evidence that the original's CPU Will strong-tosses its second serves.

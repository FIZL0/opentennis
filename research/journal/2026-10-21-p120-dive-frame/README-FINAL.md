# P120 — the dive's eligibility frame (FINAL)

## Recording
`tools/record_dive_frame.py` (new) drives P1 from slot 4 over the virtual pad. For each ball coming to P1's half it
picks a plan:
- `stop`: run across, then let go of the stick together with the ✕ press;
- `run`: keep running through the press;
- `late`: let go, then press;
- `start`: stand, then push the stick together with the press.

Each frame it samples the pad and P1's dive-check byte, search branch, dive flag and state byte (0 stand, 1 run,
2 stroke). Wall-clock timing scatters the press around the run's edge; the pad word shows what the game got.
Fixture: `context/fixtures/dive_frame_s04.bin` (run3, 5027 frames). PCSX2 slowed down under the per-frame PINE
reads, so 7200 frames didn't finish within 1000 s. The run was cut there; every sample is whole.

What the game does (49 presses that start from a stand or a run, 2 dives):
- On a press frame the check byte holds the state byte of the **frame before** the press. Press 9965 let go of the
  stick on the press frame after a run (state 1 the frame before): check 1, and the game **dove**. That is a dive on
  a run's last frame. Press 12752 pushed the stick on the press frame from a stand: check 0, no dive possible (it got a
  ground stroke).
- During the press's stroke the byte isn't refreshed: it holds for every stroke frame after it.
- A new press in a whiff refreshes it (to 2, not running). A press during a dive doesn't touch it.

## Port
- `swing::dive_run(held, running, press_live)` is the rule.
- `Player::dive_run` is taken at the start of each player's frame in `control` (before any locomotion or press) from
  `body.running` and held while a press is pending. `find_dive` reads it instead of `body.running`.
- For a human the port already searched before the frame's locomotion, so plain presses are unchanged. Two things
  change:
  - **The approach run a press plans** (`pending` > 0) no longer makes a dive eligible. The game holds the byte
    through the stroke, so it is the run state from before the press.
  - **The doubles AI's non-dive presses**, which run the locomotion before pressing, now read the frame before.
- `partner_ball.rs`'s test setup sets `dive_run` along with `running`.

Test: `crates/hst-sim/tests/swing.rs` `dive_reads_the_run_before_the_press`. For every press from a stand or a run
in the recording, the game's check equals `dive_run` of the frame before, and it is held through the stroke. Every
dive comes from a run. There is at least one dive on a run's last frame and one non-eligible press on a run's first
frame. hst-play's dive/partner tests and the netplay tests (Player is in the snapshot) pass.

## Not verified / not 1:1
- No recorded press with a planned approach run ended in a failed search. The hold through the approach is taken
  from the byte holding through every recorded stroke (the approach frames are stroke frames, state 2, from the
  press on). There is no case where the approach was the only run before a dive.
- The doubles AI's non-dive press path isn't recorded with a dive. It follows the same rule (the byte is the frame
  before the press for any press).
- The recording only covers P1 (human). The bot's dives were already checked against the frame before by
  `ai.rs ai_dives_match_the_game`.

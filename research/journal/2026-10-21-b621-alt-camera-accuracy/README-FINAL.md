# B621 — second camera accuracy

Checked the alternate camera (Select in a match) against the original in PCSX2 copy 4, slot 4's doubles, with
PINE recordings (all in `context/b621/`, git-ignored):

- `run2/`, `run3/{a,b,c}.png`: screenshots of the original with the camera on, and `run3/pp_alt.txt` a post-point
  timeline (phase, camera mode, shot number, eye) with it on.
- `alt_follow_s04.bin`: 3000 frames, per frame the match phase, the camera director's mode, the shot camera's view
  and shot number, four player matrices and the ball (scratch recorder; Select after the load, Cross every second).

## Findings

1. **Cut-aways do play with the alternate camera on.** The M7 spec said they don't (from one short point); the
   timeline shows the post-point shots (0xc, 0x61, 0x62, 0x66, 0x11, 0x98, 0x102) in phase 4 just as with the
   normal camera. On the next serve's set-up the director goes back to shot 9: the serve's first frame still shows
   the cut-away, then it cuts (no glide) to a fresh shot-9 view, settled behind the human.
   Fixed: `play/alt_camera.rs` hands the shared camera's view through while a cut-away is on screen
   (`Director::on_screen`) and starts the follow fresh on the cut back. Test
   `cut_aways_show_in_the_alternate_camera` (hst-play), two cut-aways and cut-backs.
2. **The turn after the ball lags it.** Lock-step against the recording, the original's yaw follows a ball eased
   5 % of the way to the live one each frame (from where it was at the shot's start); yaw error mean 0.046 → 0.013.
   Easing the human's spot showed no effect. Ported in `hst_sim::alt_cam::Follow` (`BALL_EASE`). Test
   `follows_through_the_rally_as_the_original` (hst-sim, 8 runs, 1611 frames compared, worst 0.025).
3. **Pull-back (not ported):** when the ball comes past the human towards the camera the eye backs away horizontally
   along the view (height unchanged), up to ~12 m, easing in and out (~0.95/frame decay). Early in a pull the 3D
   distance from the eye to the eased ball holds ~15.7 m, later it drops to ~12.7 m; no simple black-box rule fit
   (z gap / 3D distance rules left 2.5–5 m errors). 597 of the recording's frames are pulled back; the test leaves
   them out. Needs the decompile: follow-up B628.
4. **Tilt:** the original's forward height moves a little with the ball (up to 0.011), ours stays fixed. B628.
5. **Net position:** with the human at the net (z 5.20, partner serving) the base horizontal gap was 13.43 m instead
   of 11.74 m, constant through the serve. One data point; B628.
6. Each point's set-up restarts the shot (its internal clock back to 0), matched by the port's fresh follow.

## Not verified / not 1:1

- Pull-back, tilt and the net-position gap (3–5 above): not ported, black-box fits failed; follow-up B628.
- Singles (shot 8): not compared with the original (no singles save state at hand); same code path as doubles.
- The human's fade (half transparency) and the 15-tick switch glide were not frame-checked against this recording.
- Side-by-side screenshots of the remaster at the same moments were not taken; the comparison is numeric (views from
  RAM vs the port fed the same positions).

# B667 — Online: second camera doesn't work

## Cause
The alternate (close) camera (`play/alt_camera.rs`, M7/B658) was deliberately cut off online: `human`/`team` returned
`None` whenever a `Netplay` resource existed, so Select, the saved `camera.txt` choice, the camera hint, key hint and
touch button all did nothing in an online match. Online, `Pads` marks every room human as human, so simply dropping
the `online` check would have counted the other machine's players too (humans on both teams → no camera).

## Fix
- `alt_camera::locals`: the humans this machine drives — online only the players whose room seat is in
  `Lockstep::mine`. `human`/`team`/`offered` take `Option<&Netplay>` instead of a bool (callers in `camera_hint.rs`,
  `key_hints.rs`, `touch_pad.rs` updated). The split view stays offline-only (unchanged).
- Select is read from the lowest of this machine's humans' seats (online each local device sits in its seat's slot,
  `netplay::flags`), so the keyboard or pad driving the local player switches it.
- `step` steps only on a new lockstep tick (`np.lock.log.len()` grew), like `own_camera`: rollback re-runs (and frames
  with no new tick) don't advance the 15-tick glide, the rally counter or consume the press.
- Online, `OwnCam` already exists (not passive); the alternate camera's views go into `OwnCam::alt`, which `shown`
  prefers, and switching off falls back to this machine's own camera (`OwnCam::camera`), as B534 offline.

Presentation only: nothing reaches the sim or the state hash.

## Test
`alt_camera::tests::online_it_follows_this_machines_human`: singles, both seats human, this machine "g" on seat 1.
Offline `team` is `None`, online it's `[1]`; Select glides to the shot behind player 1; an extra FixedUpdate with no new
lockstep tick leaves the glide untouched (checked the test fails with the tick guard removed).

## Not verified / not 1:1
- Not tried in a real two-machine online match (unattended run, no second peer); covered by the unit test with a real
  `Netplay` + `OwnCam` setup.
- The original has no online play, so there is nothing to compare against; the camera itself is the existing M7/B658
  port, unchanged.

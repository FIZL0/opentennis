# B658: Select camera for any local human mix

## What landed
- **Who gets what** (offline, local humans):
  - One human: the original's alternate camera, unchanged.
  - Humans on one team only (2P with a CPU pair, one end): Select switches the one picture to the **doubles zoom**.
    This is `alt_camera`'s own path, now following the team's humans (`alt_camera::team`).
  - Humans on both teams, any count (1v1, 1v1 in doubles, 2v1, 2v2): the **split**. Each half sits behind its team, and
    a half whose team has two humans shows the doubles zoom (`split_view::eligible` now needs ≥1 a side, not exactly 1).
- **Doubles zoom** (`hst_sim::alt_cam::Follow::aim_team`/`step_team`, ours): the original's doubles alternate shot (9)
  with the same pull-back, ball head-room and 5 % easing. It orbits the team's ground spot: the middle of the humans
  across, at the depth of the deeper one. Then the eye backs off along the ground until every human (±0.4 m body, waist
  height) is within 95 % of the shown half-width. `width` is the share of the 4:3 width a split half really shows:
  `split` sets it from the half's aspect (a 640×720 half shows ⅔), otherwise 1.
- **Select**: the lowest human seat's (`alt_camera::first_seat`; 1P's whenever 1P plays) switches it, for both the
  zoom and the split. The key legend lists Camera wherever one of them works (`alt_camera::offered`).
- The zoom never fades anyone (the fade is one figure's). Switching off glides back to a fresh copy of this machine's
  own camera (`OwnCam::camera`, B534's camera at the humans' end), not the shared one.
- Tests: `alt_camera::tests::a_team_of_humans_gets_the_doubles_zoom` covers 1P+2P against CPUs. The match is identical
  with or without it, both humans stay in the picture with the eye behind them, there's no fade, and it switches back.
  `split_view::tests::a_team_of_two_humans_shares_its_half` replaces `two_humans_on_a_team_never_split`: 1P+2P against
  3P splits, with half 0 on players 0 and 2 and both in the picture, and the match is identical.

## Checked
- Shots in `context/b658/` (1280×720, release build). `run6/zoom.png`: 1P+2P against CPUs with C pressed, a low view
  from behind both. `run8/split.png`: `HST_SPLIT2=on` with 1P+2P against 3P, the left half behind 1P and 2P with both
  in frame, the right half behind 3P. `run7` was the same split before the width share: 2P was cut at the half's edge.
- A debug log in a real run showed the picture's eye at the zoom's view: 12 m behind 1P and 5.6 m up, against the shared
  camera at z −39 and 11.9 m up.

## Not verified / not 1:1
- The doubles zoom is ours (the original has the alternate camera only for one human). At a doubles serve its framing
  is only a little closer than the normal match camera, which already frames the near team. The difference is the low
  angle from behind. The fit constants (95 %, 0.4 m) were judged by eye on one serve set-up only.
- The court models didn't draw in this slot's `--shot` runs ("bounds" empty), the same environmental issue as M146.
  The shots show only players and HUD.
- Not driven with real pads or through a full rally on screen; the rally framing is covered only by the 400/300-tick
  tests, where the humans stand idle.
- The portrait hint (`camera_hint`, "Singles camera") still shows only for one human.
- Split with the original look stays off (as before), so there Select does nothing when both teams have humans.

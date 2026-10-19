# B580: Studio A's cloud blocked the view after M129

## What it was

The court in the user's screenshot is **Studio A Tennis Court** (`gag_c16_tvstudio`, the TV studio with balloons and
stands). The cream cloud is one of the 11 wall clouds (`c16_tv_mark02`) that the Get a Grip exporter packed into one
node, `marks2`. The one at game z −23.8, x 3.4–10.1 m, 4.1–7.5 m up, is on the studio's end wall behind the match
camera. The camera sits outside the studio at z −39.2, 11.9 m up (the wall in front of it is a `__clear` piece, hidden).

Why nothing removed it:
- **`__clear` (M8f5a3)**: the exporter splits off each island that stands between the *resting* eye and the playing
  area (doubles court + 2 m behind the baselines). From x = 0 that cloud only covers ground beyond the doubles
  sideline (x > ~3.3 at its depth), so it stayed in `marks2`, which is always drawn. The camera barely moves sideways
  (eye x stays in 0–1.3 m) but turns to follow a player who runs off court to the right, and that is when the cloud
  fills the bottom right (the user's case: "when the player moves right off court").
- **M129's shader cut**: it only acts within 10 m of the eye (full to 7 m). The cloud is 15–17 m away.

## Fix

`gs.wgsl` `cleared` (court-mod scenery only, as M129): a fragment outside the players' room, over 0.5 m up, whose
ray from the eye goes on into the room, is now also cut when it stands less than 0.74 of the way from the eye to the
room (ordered-dither fade to 0.77). This is on top of M129's 7–10 m rule (whichever keeps less).

The threshold comes from an offline pass over all 16 Get a Grip court mods (vertices at rest, eyes at x −1.3/0/1.3
behind either end; scripts were in the session scratchpad):
- cut, < 0.74: Studio A's cloud (0.72); the near end wall's beams, nets and windows on Sakuragaoka (0.67–0.74); street
  lamps right in front of the camera (Fighill 0.58, Wonderland 0.67); Tennis Fields' elderberry foliage near the
  camera (0.57–0.72, only its nearest leaves); Love Tennis' cable and plates (0.43–0.67); a few tree vertices on
  Shiranui and Tianlongshan; Fighill's walls and the University pole, which were already inside M129's 10 m.
- kept, ≥ 0.77: Dubno's house roof (0.77, three vertices), Sunflower Village's big tree (0.78), the castle gas lamps
  (0.81), the amusement park's shop front (0.77+), the stadium wall (0.79+). Dubno's antennas and rotor don't change:
  the antenna next to the camera was already inside M129's range, and the rotor is over the court.

## Checked

`--shot` with `HST_KEYS="3:KeyD*12"` (1P walks right off court) at 12 s, `HST_CLEAR=0` (before) vs 1 (after),
`context/B580/run4/*-ab.png` (before left, after right):
- Studio A: the cloud covering the bottom right is gone; nothing else changes.
- Dubno and Tennis Fields: identical apart from the players' animation (the runs aren't frame-locked).
`tools/check.sh -p hst-base gs`: passed.

## Not verified / not 1:1

- **Remaster-only addition**: the original never culls props; nothing to compare against.
- **Only three courts were looked at on screen** (Studio A, Dubno, Tennis Fields). The other newly cut pieces were
  checked offline (resting vertices, three eye positions), not in shots; a replay or cut-away camera can see them
  from other angles.
- **The user hasn't seen the before/after yet** (they offered to test live).
- **The exporter's `__clear` target** (playing area only, resting eye) is unchanged in ../opentennis-mods: widening
  it would need a re-export and new mod versions for every Get a Grip court; the shader rule covers it without that.

# B615 — Rachel's walk shoots her legs up

The walk the user saw is the post-point walk-away: in doubles, the team reactions (motions 0x30..0x34, `re_co*`)
are her donor pc12's clips (her motions.glb has only re_gu/re_di/re_gu_set/re_di_set), retargeted onto her rig with
B543's foot planting (`character::Feet::plant`). The run (motion 3) had the same fault.

## Cause
Get a Grip rigs carry `Bip01?Toe0Nub` with a 0.01 node scale. Rachel's costume skins her toe vertices to the nub
(Norma's, Wendy's etc. skin them to `Toe0`). `mods::level_feet` measures the sole in the donor's stance by skinning
each sole vertex with the joint's world rotation + translation only, so a vertex on the nub came out ×100 through
its inverse bind: her heel/toe/tip sole points landed metres from the foot (heel at (−0.27, 8.29, 11.78) in the
foot's frame). `plant`'s "toe tip never under the ground" clamp then pulled the ankle up by up to 1.2 m.

Rest axes/lengths, pelvis keys and her skeleton are the same as the other Get a Grip women (same joint nodes as
Norma); without the plant (`HST_NOLIFT=1`) her run matched stock pc12 to a few mm.

## Fix
`level_feet`'s sole skinning applies the joint's own rest scale. Packaging untouched → no mod.json bump.
Test `mods::tests::soles_on_a_scaled_nub`: disc pc12 with a scaled nub under each toe; the measured sole is the same
whether the toe vertices ride the toe or the nub (fails 4 m off without the fix). The `feet_against_the_donor`
diagnostic's `soles` now counts a toe nub's vertices as the toe's (Rachel's toe column read NaN).

## Numbers (`feet_against_the_donor`, mm, lower foot vs stock, − = above)
- run/strafe/serve walk (0,3,4,5,6,8,0x21,0x22): Rachel min −501 → −96 (Wendy, same donor: −96)
- reactions (0x2c..0x34): before, 0x32 heel −830 at 0.27; after min −21 (Wendy −21)
- every mod in out/mods after the fix: worst −146 (an umpire), no outliers

Shots (`context/B615/run1` run, `run2` walk-away motion 0x32, `--mod … --motion N`): Rachel vs Wendy, same stride,
feet on the ground.

## Not verified / not 1:1
- Looks-only mod rendering (B543 foot planting is our own addition): no original to compare against; checked
  against the same-donor Wendy by eye and by the sole-height diagnostic.
- Not seen in a live doubles match, only the viewer playing the same clips through the same `animate` path.
- Only the nub's own scale is applied (scale on an ancestor of a skinned foot joint still isn't); no rig in
  out/mods has one.

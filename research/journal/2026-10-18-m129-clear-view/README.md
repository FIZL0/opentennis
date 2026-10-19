# M129: big blockers in front of the camera are not drawn (mod courts)

## First try, rejected (2026-10-19)

The first version cut every piece of scenery standing between the eye and the players' room, stock courts included.
The user rejected the before/after shots: on Dubno Tower (`gag_c20_city`) it cut the antennas/poles and the
helicopter's rotor blades, which they want kept. Parked as `wip/M129`.

## Redo

The user's spec: only really big blockers, like the pillar that fills a replay camera on Dubno; keep the rotor and
other things over the court.

Each fragment of a court mod's `court.glb`/`gimmick.glb` is tested in the shader (`gs.wgsl` `cleared`, `GS_CLEAR`,
set by `GsKey::clear`). Stock courts never set it. A fragment is dropped when:
- it is within 1 m of the eye (a replay camera inside a prop), or
- it is within 7 m of the eye (ordered-dither fade out to 10 m), over 0.5 m up, outside the players' room's
  footprint (doubles court, 4 m beside, 6 m behind each baseline, at any height: the rotor and anything over the
  play stay), and the ray from the eye through it goes on into that room (from the ground to 20 m).

A first redo aimed the ray only at the back court; the user's replay shot showed the pillar cut only where it
covered the far half, so the target is the whole room, tall enough that a pillar's top over the skyline goes too.

The match camera sits more than 10 m from any prop outside the room, so normal play is unchanged (two Dubno match
frames, `HST_CLEAR` 0 vs 1: only animation differs). `HST_CLEAR=0` turns it off for A/B shots. `HST_KEYS` gained
`KEY*S` (hold a key S seconds) and KeyW/A/S/D.

Shots: `context/m129/show2/` (in the `opentennis-m129` worktree). The user checked it live and approved the merge.

## Not verified / not 1:1

- **Remaster-only addition**: the original never culls props, so there is nothing to compare against.
- **Only Dubno was looked at.** The user remembers a second court with a blocker but not which.
- **Players and the ball aren't tracked**: the shader has no per-frame data; the rule is geometric only.
- **The cut leaves an edge**: a cut prop shows a hole, softened by the dither between 7 and 10 m.

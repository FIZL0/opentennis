# M158 — Sophie's hands-to-mouth celebration misaligned

## Which clip
The reference video (*All Sophie Animations*, 34–36.5 s) shows Sophie's palms framing her face, wrists together
under her chin. That is her source clip `ku02_sh`, packed as `re_gu.4` (5th take of the point celebration). `ku02_cl`
(the clap, `re_gu.3`) is the other close-contact one.

## Where the offset came from
- **Retarget/rerig: fine.** The wrists in `motions.glb` on the costume vs the source over the clip: within ~5 mm
  (×0.97 body scale); the game's own sampled pose (`pose_at` on the loaded mod) gives the same numbers (hands
  0.10 m apart, 0.23 m in front of the head joint at frames 56–68, as the source). An unmirrored costume and the
  pre-mirrored one draw alike, and the match's left-hander mirror is a plain x flip (it keeps distances).
- **Our player: the off-hand racket hold.** `animate` pulls a mod's off hand onto the racket where the donor's
  would be (B676 grip), except on the mod's own clips. Its "own clip" check only looked at the first take of each
  motion (`visual`), not the further takes (`re_gu.1`… from M130). So on every extra take the hold ran at full
  weight whenever the hands came near the racket hand: clapping or framing the face. On `re_gu.4` it moved her off hand
  5–6 cm and turned it to the donor's grip, across her mouth. The racket is hidden during that clip anyway
  (`racket: false`).

## Fix
The hold's own-clip weight now uses `CharacterData::own` (first take and every further take), as the feet code
already did. Nothing in the mod changes, so no `mod.json` bump and no re-upload.

Side by side (level camera, frame ~66): reference vs before vs after: `context/m158/cmp2.png`. Before, the gloved
off hand covers her mouth; after, both hands frame the face as in Out of Bounds.

## Other clips and characters
A sweep over every Out of Bounds mod's takes (the hold's pull at full weight, sampled every 4 frames;
`context/m158/sweep.txt`) found 169 takes where the off hand was pulled 1–20 cm (Sophie's up to 12 cm,
Fernando's up to 20 cm). All of these are takes, so the one fix covers them all. Golf-game mods from the other
games use the same takes path.

## Tools added
Character viewer: `HST_TAKE=K` shows a mod's K-th further take of `--motion`; `HST_VIEW_PITCH` sets the camera's
pitch (with `HST_VIEW_Y`/`HST_VIEW_YAW`), e.g.
`HST_VIEW_PITCH=-0.05 HST_VIEW_Y=1.15 HST_VIEW_YAW=3.14 HST_TAKE=4 tools/shot.sh out.png --mod <mod> --motion 44 --shot-at 1.1 --radius 2.2`.

## Not verified / not 1:1
- No in-match shot of `re_gu.4` itself: the match picks the take from the motion-set count, and forcing it wasn't
  worth new code. The match draws the same `animate` path as the viewer (head look is off in reactions, the mirror
  is a flip), and the probe measured the hold's pull on the loaded mod directly.
- The viewer draws the racket during `racket: false` clips (the match hides it). That's viewer-only.
- Rendering is checked by eye against the video (PLAN: presentation only has to look right).

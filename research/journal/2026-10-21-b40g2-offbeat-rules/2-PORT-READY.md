# B40g2 port, concept round 1: Offbeat Rules on `concept/offbeat-rules`

Port phase from `SPEC.md` with the decompile locked. All the code is on the local branch `concept/offbeat-rules`
(two commits plus a merge of main; it is in the shared repo, so any checkout can `git switch` to it). It has **not
been pushed**: the runner prompt says never push, which overrides AGENT.md's "push the concept branch". Push it
with `git push -u origin concept/offbeat-rules` once you've looked at it. This task branch carries only this journal.

## What's on the branch

- **Irregular bounce** (the original's rule 1, ported): every launch (strike, serve toss) draws its two uniforms
  from the shared generator, as the game already did. With the rule on they turn each in-play bounce of that flight:
  pitch = 1° × (u₁ × 15), yaw = (2u₂ − 1) × 1° × 15 (20 for a lob). The turn is applied before the serve's bounce turn,
  in both the swept and the plane step (`ball::irregular`, `Flight::irregular`). The ball is drawn as
  `ball_irregular.mdl` from CMN/GAME.XB. The court's bounce sound is key 13, never slowed (`sound::Bounces::irregular`).
- **Slow motion** (the original's rule 2, ported): from the serve's set-up to the point's end, each odd tick of the
  point gets one in-between frame. That is the clock at ½ after odd ticks, through `Time<Virtual>` as the instant
  replay paces it, so the ticks themselves are unchanged.
- **Turbo rally** (ours, a prototype): the rally's clock speeds up 10 % for each shot after the return, up to ×2.
  Slow motion and turbo multiply.
- **Ghost ball** (ours, a prototype): the ball and its outline are hidden from 1 m past the net until the first
  bounce. The shadow stays.
- **Menu**: on the local confirm screen, Select / − (keyboard C) opens an "Offbeat Rules" box with four On/Off pills,
  driven with ↑↓, ✕ or a tap, and closed with ○. The legend shows "Rules (n on)". The match gets `--offbeat a,b,…`.
  It is hidden online. The `HST_MENU=confirm-offbeat` preset shows the box, and it was checked at 16:9 and a tall
  window (screenshots in `context/b40g2/shots/`).

## How to try it

`git switch concept/offbeat-rules`, build, then either use Local → confirm screen → Select/C, or run
`hst <iso> --play --offbeat irregular,slow,turbo,ghost` (any subset).

## Verification

- `hst-sim/tests/offbeat.rs`: two PCSX2 recordings with rule 1 forced on in the rules word
  (`scripts/b40g2_irregular.py`, `context/b40g2/run2/{r1_b2,r1_long}.bin`, 1,500 + 9,000 frames). The port matched
  the game bit for bit on 7,239 ball frames, 101 of them deflected bounces (lobs included, which confirms the 20°
  spread). The uniforms were found in the recording's ball object by search; exactly one pair of words reproduces
  every deflection.
- `hst-play` `offbeat::pacing_and_ghost_leave_the_match_alone`: two bot matches of 4,000 ticks, one with
  slow/turbo/ghost on and one with every rule off, end in an identical `Game`. The test also checks the slow pattern
  (½ after odd point ticks, 1 between points), that turbo builds up, and that the ghost hides the ball.
- Mode off: `hst-sim` replay and live tests still pass on the merged branch. The slow-motion recording is
  `scripts/b40g2_slow.py` (from the spec phase), with its output in `context/b40g2/run1/slow.txt`.

## Not verified / not 1:1

- **Rule 1's ball tint**: the per-court tables that tint the irregular ball are not ported, so the ball keeps its
  model's own colours. They could be read from GAME.BIN through `hst_data::exe` if the rule goes to main.
- **The irregular ball model and bounce key 13** were not compared with the original by eye or ear. The model loads,
  and the key comes from each court's bank.
- **The CPUs' predicted path under rule 1** runs the same `Flight`, so it sees the turns. It was not compared with the
  game's predictor object in the recording.
- **Rule 2's in-between frames** come from our renderer's interpolation. The sound pitch and crowd during them
  weren't checked. The arming of the match's very first serve may be off by one tick: only later points were
  recorded.
- **Online**: the room doesn't carry offbeat rules, so the rules box is hidden online.
- **Four unaccelerated ball frames** in `r1_long` (a serve at frame 5 and a rolling ball after its point, two frames
  each) miss even without the rule. This gap predates this task; the test allows them and a follow-up task is filed.
- **Turbo rally and Ghost ball** are our own design, with no original to match.

## Next

The user tries the branch. Then either another concept round (tune turbo's step, ghost distance, tint tables, more
rules from the proposal) or a "merge B40g2 concept into main" task.

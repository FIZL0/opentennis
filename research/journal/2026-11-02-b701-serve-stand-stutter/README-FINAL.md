# B701: stutter when the server stands still after a serve

## What the original does (sim state, from `fixtures/anim_s05.bin`)
`research/journal/2026-11-02-b701-serve-stand-stutter/scripts/orig_handoff.py fixtures/anim_s05.bin` prints the original's motion player around every serve → stand
switch (three in slot 5's recording, all the same shape):

    v7838 motion 0x25 sampled 61   (serve clip, length 61, clamped at its end)
    v7839 motion 0x25 sampled 61
    v7840 motion 0x00 sampled 0    fade from 0x25 at 62, weight 0.750
    v7841 motion 0x00 sampled 1    fade weight 0.625 … v7846 weight 0, v7847 fade gone

So: the serve's end pose held for two frames, then stand from frame 0 crossfaded (8 frames) out of that frozen pose.
Our sim gives the same states (the clock is bit-exact, `anim_s05_clock`; our logs show the same 61, 61, then stand),
so nothing was wrong on the sim side and the sim is untouched.

## What we drew (the stutter)
Our drawing layer (looks only, `character.rs` `animate`) turns the serve's follow-through into the ready stance from
clip time 22 to 44, the stance bobbing on its own clock (frames since the swing started − 22). At the hand-off the
real stand motion restarts at time 0: a different point in the bob. The 10-frame `SERVE_EASE` hid the position jump,
but:
- the body stopped dead for a tick: the new motion's first drawn tick is frame 0 to frame 0 (drawing runs a tick
  behind the clock), and the ease starts flat;
- the bob reversed (head going down turned up, then down again);
- the racket arm swung back at up to 3.3 m/s: the serve's contact IK stays at its contact weight through the
  follow-through (covered by the ready stance) and the stroke IK lets it go over ~4 ticks once the server stands.

Measured with `HST_POSE_LOG=1` (new: `animate` logs each figure's drawn frame) on
`hst … --play --singles --chars 0,1 --pads -,-,-,- --shot … --shot-at 23` with `HST_WEATHER=0` (character 0's serve at
20.3 s, stand set at 21.27 s, same match every run), `research/journal/2026-11-02-b701-serve-stand-stutter/scripts/pose_vel.py` (speeds per drawn frame) and
`research/journal/2026-11-02-b701-serve-stand-stutter/scripts/plot.py`.

## Fix (drawing only)
The stand set right after a serve keeps the tail's clock: its drawn time runs `(frames from the swing's set to the
stand's set) − 22 + 1` frames on from its own, and its first tick moves on from the frame before it instead of
sampling frame 0 twice. When the stance was already drawn whole (it always is when the serve played out), there's no
`SERVE_EASE`. While it plays, the serve's leftover arm IK isn't drawn on it. If it later fades out into a run, the
outgoing pose is drawn on the same clock. Runs out of the serve (stick) keep the ease as before.

Strip: `context/b701/handoff.png` (head height, head speed, racket speed per drawn frame, old vs fixed, same serve).
Old: head speed 0.6 → 0 at the set, bob reverses, racket 3.3 m/s. Fixed: continuous. The dip at 21.32 s in the fixed
line is the stand clip's own turning point (old shows it at 21.48 s). Logs: `context/b701/run2` (old), `run4` (fixed).

## Not verified / not 1:1
- No screenshots of the original's hand-off: PCSX2 copies can't screenshot while paused, and a real-time F8 lands
  hundreds of ms late, so a frame strip wasn't practical. The original's side is its recorded per-frame motion
  state (above) instead.
- The look isn't the original's on purpose. The original snaps from the frozen serve end into stand frame 0 over
  its 8-frame crossfade; ours keeps the earlier looks-only choice (follow-through turns into the bobbing stance) and
  now only carries it on smoothly.
- The stand → run fade-out on the carried clock is reasoned, not seen: in the CPU runs (`--chars 0,1` and `3,5`, ~2 min) every
  stand after a serve ended at the next point's cut. A human server who stands and then runs would show it.
- No automated test: `animate` is a Bevy system with no test harness for drawn poses. Checked with the pose log.

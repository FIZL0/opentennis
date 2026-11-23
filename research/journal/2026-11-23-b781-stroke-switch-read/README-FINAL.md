# B781: when the stroke switch takes effect in the stand-test read

## Task

B779 reported a miss at the European racket-z fit. At ticks 179 and 538 (B773's numbering, rally window vsyncs 38056 to 39692), player 1 switches from the turn_fore motion to a stroke motion (0x12 at 179, 0x10 at 538). Play's doubles stand-test read already shows the new motion on the switch tick (179: -0.8610), while the game's read still shows turn_fore one frame longer (-0.7908).

Question: on which tick does the game's stroke start take effect, as seen by player 2's read, and can play match it?

## Method

Two temporary, env-gated prints were added and then reverted. No code change landed.

1. Inside the stand test's own read (`scratch::z`), for player 2: print player 1's kept motion, its motion command, its pending after-step swing, and the z value it read.
2. A recomputation of player 1's racket read inside `scratch::after_pose`, where B773 and B779 had put their print. This runs before that system updates the kept pose.

Run: `HST_B781=1 cargo test --release -p hst-play --lib the_european_bot_match_plays_as_recorded -- --ignored --nocapture`. The logs were converted to B773's print format and fitted with `research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_lead_fit.py` against `context/b772/run6/eu_stack.bin` over vsyncs 38056 to 39692. Logs and fits are in `context/b781/run1` (base.log, base.conv, base.fit) and `context/b781/run2/log`.

## Finding

Play already matches the game. Nothing needs to change.

- The game's waiting swing is the stroke motion that sits behind the body turn, started 8 frames before contact. It shows from the next motion step. Play does this already: B773's swing-after-step rule notes the swing during player 1's swing update, and it lands in the motion command after the tick's motion step.
- On the switch tick, player 2's real read still sees player 1's command as turn_fore, because the swing is still pending. So it reads turn_fore one frame longer: -0.7907854 (game -0.7908).
- The next tick it reads the new motion's first frame: -0.860981 (game -0.8610). Then -0.9508904 and -0.9948613 (game -0.9509 and -0.9949).
- Same pattern at 538. The real read on the switch tick is -0.7895945 (game -0.7896), then -0.75290996 and -0.7521255 (game -0.7529 and -0.7521).

Why B779 saw a miss: its print was a recomputation in `after_pose`, not the AI's own read. By that point the swing-after-step system has already put the stroke into player 1's command, while the kept pose is still the previous tick's. So on the switch tick the print plays the stroke from frame 0 one tick early and shows -0.8610 twice. The `after_pose` recomputation in run2 reproduces B779's sequence exactly (-0.7902, -0.8610, -0.8610, -0.9509). The real read gives -0.7902, -0.7908, -0.8610, -0.9509.

This is not new behaviour since B777. It showed up then because the print and the real read only disagree when a waiting swing starts.

## Results

Real-read fit, run1:

- 558 of 576 game ticks fit at shift 0, including ticks 177 to 182 and 536 to 541.
- Remaining misses:
  - ticks 7 to 14: B779's in-fade residual, which waits on B778's idle clock;
  - ticks 69, 188, 392 and 547: trail colour reads, which belong to B776.
- Ticks 1 to 6 fit at shift 1. This is the opening idle offset that B779 mentions.
- Some flat turn_fore stretches (for example ticks 58 and 59) fit two shifts, because neighbouring values are equal.

Advice for later fits: print the read inside the stand test (`scratch::z`), not in `after_pose`. A print in `after_pose` misses waiting-swing starts.

No code changed. No release note.

## Not verified / not 1:1

- No new PCSX2 capture was taken. The B772 run6 stack capture was used, and it holds the game's own read.
- The in-fade ticks 7 to 14, the trail-colour ticks and the opening idle shift were not examined. They belong to other tasks (B778, B776, B779's in-fade residual).

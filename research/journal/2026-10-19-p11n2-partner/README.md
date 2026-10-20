# P11n2: doubles partner logic (spec phase)

The spec is in [SPEC.md](SPEC.md). Status: the spec is done (`--spec`), and the port session goes next.

## Findings
- The task line says the app's "nearer by x" stand-in (`bot`'s `mine` filter and its call-out) decides who plays a
  doubles ball. That block can't run in a real match:
  - P11k3 routes every receive and rally frame of a computer player through `doubles_ai::step`;
  - on every other frame, `bot` returns early or has no interception plan.

  Who plays the ball is decided by the ported routines: shot records, give-way, follow, middle pick and voice
  (P11k2), plus the claim and the human record keeper (P11k7).
- Three inputs to that logic aren't wired in the app:
  - the partner's movement state, which gates the give-way voice and its AI draws;
  - the player's own one-frame miss mark, which posts a "missed" record so that a following partner takes over;
  - the claim's exemption from keeping 1 m apart while stroking. The app's contact-step ponytail ("partner check
    skipped for the server") is this rule.
- Scan of `context/fixtures/match_s05.bin` (four CPUs, 26 400 frames):
  - teammates are never both locked after the first set-up;
  - both pressing at once happens 3 times;
  - 6 miss marks, each 1 frame, all from presses that found nothing;
  - teammates never come within 1.52 m, so keeping apart isn't exercised.
- `1p3goodcpus.bin`: 1 mark; teammates sit at 1 m only while running or standing.

## Method
- Read the doubles rally routines' swing step, the give-way voice, the claim setter and reader, the stroke-state
  movement update, the miss routine and the contact search's resets, and their callers.
- Checked the ported Rust (`hst_sim::rally`, `doubles_ai.rs`, `play.rs`) for which parts are already wired.
- Scanned the two fixtures with a throwaway script in the scratchpad. It isn't committed: it reads only the
  fixtures and prints counts.

## Tool note
`research/xref.py` fails inside a `.claude/worktrees/…` checkout: it builds the `.funcs.tsv` path with
`path.replace('.c', …)`, which also rewrites the `.c` in `.claude`. Run the main checkout's copy, or fix it to
replace only the suffix. `research/fn.sh` works once `context` is symlinked into the worktree.

## Spec pass 2 (2026-10-20): open questions answered
- **Player update order:** confirmed from `ai_net_s05.bin`. Each player runs its AI and then its own movement update,
  in index order:
  - the AI calls come in ascending player order;
  - players 2/3 see their partner's position and state after its move on that tick;
  - players 0/1 see them from before it.

  The app's `control` → `bot` does the same.
- **Stroke-state keep-apart:** missing in the app. Only the contact step's IK tick, the run/stand mover, the
  approach and the dive pass the partner. The serve IK tick passes none. The spec now says what the port adds.
- **Exemption proof:**
  - In the decompile, the push routine takes a "may push" flag. The three stroke-time movers set it to "not the
    claimant".
  - A rule test in the port is the acceptance. No capture is required, and the port lists that under *Not verified*.
- The instant replay's save/restore skips the stroke check and measures against the partner's saved position. This
  is replay only, nothing to port.
- Scan scripts are in the session scratchpad, not in git. They read only the fixtures.

## Spec pass 3 (2026-10-20): brief fix only
- The spec had no open questions left, but pass 2's "Resolved in this pass" notes sat under `## Open questions`.
  `tools/ctx.py` reads any text there besides `none` as an open question, so it kept handing out a spec phase.
- Moved those notes to their own section, *Answered questions (spec pass 2)*. The brief now gives the port phase.
  Nothing in the behaviour changed, and the decompile wasn't read this pass.

## Not verified / not 1:1
- Nothing new this pass. The port's list is in SPEC.md *Verification* (the claim exemption has no recording of the
  original).

## Port pass (2026-10-20)
Ported from SPEC.md, with the decompile locked. Code is in `crates/hst-play/src/play/doubles_ai.rs`, with small hooks in
`play.rs`. Tests are in `play/doubles_ai/handover.rs`.
- **Voice gate:** the step passes the partner's movement state (stroke = 2, running = 1, hit by ball = 3) as it is
  at the moment of the step.
  - The sim never armed the "may call" flag, so call-outs never happened. Arming came from the fixtures (SPEC.md
    *Port findings*): it re-arms at the point's set-up and on the opponents' hit.
  - A four-bot run now gives 18 call-outs from players 0/1, none of them while the partner was stroking.
- **Miss mark:** a whiff (a press with nothing to find) or a cancelled lock playing its miss marks the player. Its
  next AI step takes the mark and the swing step writes its record as "missed" (HITTING). The mark is cleared by a
  new press or a serve swing.
- **Claim exemption:** the stroke states get a per-frame zero-step mover check, and the contact IK step and the
  serve IK tick now pass the partner, unless this player holds the claim. A non-claimant's IK may not move its body
  freely. A serve swing's start takes the claim.
- **Stand-in removed:** the old `mine`/`intercept`/call-out block in `bot` could not run, because the receive/rally
  routine always takes those phases for 2 or 4 players. It was deleted along with its helpers and two player fields.
- **One player per ball:** an 8-minute four-bot run has 4 frames in 2 episodes where teammates are both locked. In
  both, the partner found the ball on a later tick and took the claim; the spec says that is allowed (both can lock,
  and the first strike cancels the rest). The test checks that the first strike cancels the other lock, and caps
  the count.

## Not verified / not 1:1
- Claim exemption: there is only a rule test. There is no recording of the original with teammates that close
  (match_s05 never gets them within 1 m), and it needs a scripted b479-style run.
- Serve swing: the keep-apart is checked only in the serve IK tick, which is the app's only body move during a serve
  swing. It is not compared against the original.
- "May move freely" is read as the arm IK's body-move permission (`ArmIk.free`). There is no recording to confirm it.
- Miss mark timing: the mark is taken before the app's own pre-step whiff update, so a miss is seen on the following
  frame's step. This follows the spec's order, but it is not checked against a frame-exact recording.
- Voice arming comes from the fixture scan, not from the spec text. It matches all 11 doubles fixtures apart from
  cross-point cases.
- One player per ball: statistical only (4 frames / 2 episodes against match_s05's 0 frames and 3 both-presses).
  The ball-for-ball comparison needs the same-state doubles replay (P11k6).
- The voice-gate glue is asserted for players 0/1 only. Players 2/3 step after their partners, so they read the
  partner's state as already updated this frame, as the spec says.

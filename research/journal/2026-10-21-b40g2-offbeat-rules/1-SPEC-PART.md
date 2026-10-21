# B40g2 spec phase: Offbeat Rules, round 1

Rules 1 and 2 have to come out of the decompile, so under the clean-room rule (PLAN.md) this session wrote no Rust.
It wrote `SPEC.md` and the proposal below. The next session ports both rules from the spec on `concept/offbeat-rules`
(no branch yet; create it from main). It also prototypes the proposal's pick or picks behind the same rules page,
pushes the branch, and ends with `--blocked "concept round 1 on concept/offbeat-rules: waiting on the user"`.

Findings, in short:
- **Rule 1 (irregular bounce):** every bounce in a rally turns the ball's new velocity by the shot's two launch
  uniforms (already drawn and dropped by the port): down by 0–15°, sideways by ±15° (±20° for lobs). It uses the same
  turn the serve's first-bounce turn uses. The CPUs' predicted path sees the same turn. On top of that: a different
  ball model, a different per-court ball tint table, bounce sound key 13 instead of 0.
- **Rule 2 (slow motion):** armed at each serve phase's start, off when the rally ends. It flips a ×2 slow-motion
  pace on and off every tick, so ticks alternate 2 and 1 frames, about ⅔ speed. Sim untouched. Recorded on PCSX2
  (`scripts/b40g2_slow.py`, `context/b40g2/run1/slow.txt`).
- Rules 3/4 are placeholders: help text only, nothing reads them.

## Proposal: new offbeat rules

The aim: things that make a casual match funnier or more chaotic without breaking the game's feel, and that keep
mode-off bit-exact. "Sim" says what the bit-exact sim would have to change. Anything that changes the sim must also
go into the online handshake (`hst_base::handshake`), like rule 1.

| Rule | How it plays | Sim change | Cost |
|---|---|---|---|
| **Turbo rally** | The longer the rally, the faster the game runs: +10 % per shot after the serve, up to ×2. Long rallies turn frantic; reaction matters more than reach. Rule 2's opposite. | None. Pacing only, like rule 2 and the replay pace (`Time<Virtual>` speed). Ticks, inputs and results are the same. | Low. Online: both peers' clocks must move together. It derives from the shared shot count, so it is deterministic. |
| **Ghost ball** | The ball fades out after it crosses the net and shows again at its first bounce. Only its shadow (and the lob marker) tell you where it is. Reading shadows becomes the game. | None (rendering). | Low: the ball draw's alpha from `Flight`'s side/bounce state. |
| **Gale** | Every game is played in the strongest wind the court allows, from a new direction each game. Lobs drift, drop shots stall. | Weather schedule override: wind kind/gust forced to full at schedule time. The shot and AI code already handle wind (`Shot::wind`). The RNG draws must stay the same count, i.e. override the result, not the draws. | Medium. Needs a check that the CPUs' path prediction handles the strongest wind on every court. |
| **Big ball** | A beach-ball-sized ball, ×2 radius and more drag. Slower, floatier rallies, easier for beginners. | `ball::Params` radius/drag per match. The CPUs' predicted path uses the same params, so they stay consistent. The trajectory tables assume the normal ball, so shots land short. Aim needs a correction or it reads as part of the fun. | Medium. Tuning is the work. |
| **Iron legs** | Stamina drains 3× faster. Long rallies leave players crawling, so the game rewards ending points. | `player::drain` scaled per match. The AI already reads tiredness. | Low. |
| **Sticky court** | Every bounce keeps only half its height: low, skidding balls everywhere. | `Surface` restitution per match. | Low–medium. The AI's predicted path follows; bounce-height balance needs tuning. |
| **Shrink ray** | Each point lost shrinks your player 10 % (handicap-like run-speed loss and smaller model), and each point won grows you back. | Per-player `size` in `run_speed` (B40g1's factor) changes between points; model scale. | Medium. Shares B40g1's port. |

(Golden point / sudden-death deuce is already a setting: `Rules::no_deuce`.)

**Pick for the prototype: Turbo rally** (first) and **Ghost ball** (second). Neither touches the sim, so mode-off
and mode-on replays stay exact for free, and both are cheap. Turbo rally pairs naturally with the original's slow
motion as its opposite. Ghost ball is the most visibly "offbeat" one. Gale is the best sim-side candidate for a
later round.

Rules-page layout for the branch: Irregular bounce, Slow motion, Turbo rally, Ghost ball (on/off each), in the
confirm screen's offbeat-rules entry. Slow motion and Turbo rally can be on together (multiply the paces); say so in
the help text, or make them exclusive.

## Not verified / not 1:1

- Rule 1's deflection is read from the decompile, not checked in lock-step. The port checks it with the function
  oracle: the turn and the angle step are pure functions of the ball and two floats, so ≥10k generated inputs
  work. Ideally it also checks a PCSX2 recording of a rally with rule 1 poked on before the match's ball is made.
- Rule 1's model, tint and bounce sound key come from the decompile and the disc. They weren't seen running. The
  port compares by eye and ear against the original with rule 1 set on its confirm screen.
- Rule 2's pacing was recorded (slot 5, rule poked on mid-match: serve, rally, post-point, next serve). Whether
  sound pitch or the crowd change during its in-between frames was not checked (audio capture). Presentation, by
  ear.
- The proposal's rules are our own design, not the original's.

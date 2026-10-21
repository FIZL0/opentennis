# P11l6 — port phase (decompile locked)

Ported from `SPEC.md`. No decompile read.

## What changed

- **Shot target** (spec §1): `doubles_ai::aimed` keeps each player's own target, set where the app aims:
  - rally shots: `shot::aim`'s `target`, in `aim_target`;
  - serves: `serve::target`'s aim, before `serve::inside` and the scatter.
  It's stored per player as (x, y, z, 0) in `Shared::targets` and goes into the snapshot. Singles `Body::target` reads
  it. `landing()` (the flight's next landing) is gone.
- **Shot memory spot** (§2): `singles_hit`, run from `heard(msg::HIT)` at the strike. For each CPU on the other side it
  sets `opp_kind` and `last` = the hitter's position then (y down as stored, w 1), and runs the dive dash
  (`Rally::heard_hit`). It's no longer done at the AI's next `step`, from the opponent's position.
- **Hit order** (§3), singles only:
  - `heard_hit` no longer arms the early return-to-centre roll, so `ai_heard_shot` makes no net repick.
  - `step` doesn't re-enter the rally or the receive on a new shot. Only strike/new path (`struck`) and a change of
    state still re-enter.
  - `heard` no longer clears `held` on the hit; new path in the rally still clears it.
  Doubles is unchanged: re-entry from shot 2, receive re-entry on any new shot, `held` cleared on the hit.
- **under_min** (§4): new `Game::under_min`, taken from the character's TParam row (`ReachStats::from_tparam`). A mod
  character gets its own row's value (`mod_match::apply`). It goes into `Body::reach.under_min`.

## Tests

- `doubles_ai::tests::singles_wiring` plays a bot singles match (40 s) and checks:
  - under_min equals TParam's for both characters;
  - no early roll is ever armed;
  - each shot's target has y and w at 0, and the other player's target is unchanged by the shot;
  - the opponent's mark has w 1 and the hitter's kind, at the hitter's x/z.
- The existing doubles_ai tests and the netplay tests (`HST_NETPLAY=1`, Shared is in the snapshot) pass.

## Not verified / not 1:1

- **The done check (the frame-exact singles `step` replay) wasn't run.** It also needs P11k6's shared gaps (contact
  depth, smash offset, path mode/gap, hold flag, stick quad), and P11k6 is still open. Per the spec's *Scope*, this
  port covers only its own four items. The replay check moves to P11k6 (noted on its line).
- **The kept target vs the recorded one wasn't compared call by call.** That needs the app to replay the recorded
  point (the same dependency). The test checks the target's shape (y and w 0, per player, kept across the opponent's
  shot), not its values against the game.
- **The spot's x/z are checked only to 0.2 m** against the hitter's position after the tick. The hitter can still
  move in its own update after the strike. The value is taken at the strike itself.
- **The serve target's y** is whatever `serve::target` gives (0 from its shared aim), not forced to 0.

## Side effect on a test

`match_stats::hold::the_match_end_holds_its_last_picture` plays a bot singles match. It forced 40–0 at every serve
and needed player 0 to win two games within 2 minutes. The singles AI's changed play gives longer rallies, so player 0
won only one in that time. The test now forces games 1–0 as well, so the first point player 0 wins ends the match. The
test checks the same things, and no longer depends on how the bots happen to play.

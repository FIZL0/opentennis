# M139 — in-match controller rumble (ours)

The original never rumbles in a match (M138); the user wants it anyway. Presentation only.

## What it does
`crates/hst-play/src/play/rumble.rs`, one `Update` system that reads `Game` after the ticks and writes
`GamepadRumbleRequest`s; it never writes to the match.

- **Hit** (`Game::shots` grows; hitter `last_hitter`): strength and length by the ball's launch speed
  (`hit_flight` velocity, 0.3 → 0.75 m/tick mapped to 0..1; a bot match's hits run 0.27..0.75, median 0.55),
  +0.15 on the sweet spot (offset 0). Big motor t², small 0.35 + 0.5t, 70–180 ms. A framed hit or a rally hit two
  frames or more off the sweet frame: a light 50 ms tick on the small motor only.
- **Ball hits the player** (`body_hit`): 0.8/0.6, 250 ms.
- **Side wins** (when the players react, or the match call starts): point 0.15/0.5 350 ms; game or set 0.45/0.7
  600 ms; match 0.8/1.0 1.2 s. Every player of the winning team.
- Each event once: max shots seen per rally (a rollback's re-simulated ticks don't re-buzz); the win keyed on the
  score it happened at, so an instant replay that comes back to the same point's end doesn't buzz twice.
- Only humans of this machine (`Pads::slot_of`; online also in `Netplay::lock.mine`) on a real pad (`PadSlots`; not
  the on-screen touch pad). Nothing during an instant replay (`instant.replaying`) or a played-back match
  (`tick_input::Replay`). Bots: `slot_of` is None.
- Gate: `rumble::ON`, stored by the menus from Controller Vibration (`pad_vibration`) where Key Hints' is stored
  (main menu `fit_grid`, pause menu, settings load). Its description now mentions the match.

## Tests
`tools/check.sh -p hst-play rumble`: strengths stay in range and order; a 2-minute bot match (2.6 s): every hit
buzzes its hitter, only the winning side buzzes for a point, the same state twice buzzes nothing.

## Not verified / not 1:1
- Not 1:1 by design: the original has no in-match rumble; strengths/lengths are by feel (the task says so).
- Not felt on a real controller: this run is unattended with no pad to hold. The request path is the same
  `GamepadRumbleRequest::Add` M138's menu buzz uses, which works.
- The routing (human → seat → pad, online `mine`) has no automated test: a bot match has no humans, and a
  human seat would stand idle. It reuses `slot_of`/`PadSlots` exactly as `read_input` does.
- Game/set/match buzzes and the body-hit buzz aren't reached by the 2-minute test (a game needs a longer run than the
  10 s test budget); their logic is the same branch as the point's.
- A framed hit's flag lasts a tick; if two ticks run in one display frame it may read as a clean hit.

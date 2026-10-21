# B622 — no camera switch while movement is locked

The alternate-camera toggle (Select, `camera` bind, `play/alt_camera.rs`) already ignored presses in the change of
ends and in replays. Now it only switches in live play: phase Serve or Rally, no post-point cut-away on screen, no
replay (`alt_camera::live`). A press outside that is dropped on the next tick (not kept latched for later). During the
court intro the fixed loop is held, so `press` no longer latches at all while the intro runs (`intro::running`),
otherwise the press would land on the first serve tick.

Presentation only: the toggle writes nothing the sim reads; the existing M7 test (match identical with and without
the plugin) still passes. New test `select_is_ignored_while_play_is_stopped`: Post, ChangeEnds and replaying each
drop the press; Serve switches.

## Not verified / not 1:1
- The original's own rule for when Select is taken wasn't checked against PCSX2: this follows the user's wording
  (only while the players can move). Presentation only, so no PINE proof is required.
- The cut-away case (`Director::showing`) isn't forced in the test (the shot field is private to `cutaway`); it is
  covered by the same `live` check.

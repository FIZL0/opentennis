# M138 — controller rumble like the original

## What the original does
- **It never rumbles during a match.** The pad library's two actuator calls (set-alignment, set-direct) have no
  caller in the base program or the match overlay: no call instruction and no stored pointer to either of them.
  Only the menu overlay calls them.
- Black-box check: `tools/vpad.py` now advertises FF_RUMBLE and logs every effect PCSX2 plays on it
  (`$XDG_RUNTIME_DIR/hst-vpad<N>.rumble`). In 25 s of the bot game (slot 5) and 60 s of slot 3 with P1 driven
  (Cross mashed, stick swung), the log stayed empty. Writing an effect to the pad by hand logged it, so the logger
  works.
- Menus (the menu overlay) only use the small motor, which is on/off, and everything is gated by the game's own
  Vibration option:
  - switching Vibration on buzzes each pad that pressed for 15 frames (0.25 s);
  - a pad that takes its place on one of the player/pad screens buzzes 15 frames;
  - one pad screen pulses a pad for 290 frames while a button pair is held: 10 frames on / 10 off, or
    10 on / 20 off, depending on which button.

## What we do
- No in-match rumble, the same as the original. The task assumed shots, smashes and points would rumble; the game
  has nothing to port there.
- I split the switch from the old one. Settings → Mobile → Vibration is the touch buzz and sits on a tab that only
  touch screens show, so on a desktop with a pad that one switch would read wrong. The new switch is
  **Settings → Display → Controller Vibration** (`pad_vibration`, on by default), next to Button Icons. It gates
  B564's Assign-screen press buzz (`assign_shake::flash`). Switching it on gives the pad that pressed a 0.25 s
  small-motor buzz, like the original's option (`assign_shake::vibration_on`).
- The menus are ours to design (PLAN.md), so the original's 290-frame pulse screen isn't copied. Our Assign buzz
  (B564) stands in for its seat buzz.
- Tests: `assign_shake::a_press_flashes_its_card_and_rumbles_its_pad` (off: the card still flashes, no buzz),
  `assign_shake::switching_vibration_on_buzzes_the_pad`, `settings_round_trip` (default on, round-trips off,
  independent of the touch switch).

## Not verified / not 1:1
- The rumble logger was not checked end to end through PCSX2: I didn't drive the original's options menu to switch
  Vibration on and see the 15-frame buzz land in the log. So the empty in-match log rests on the decompile finding
  (no actuator calls in the match code), not on the logger alone.
- The original's Vibration option state in save states 3/5 wasn't read. It doesn't matter for matches, which have
  no actuator calls at all.
- I didn't feel the real rumble on a physical pad: `--shot` runs read no gamepads (P62), and unattended runs launch
  no live game for the user. Only the unit tests cover it.
- The original's 290-frame pulse pattern and its seat buzz are not 1:1. They belong to the front-end, which is ours.

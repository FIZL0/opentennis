# M189: the pushed stick at a computer player's contact

## What it was
- Every receive and rally routine holds its own stick out in its swing state at swing 1, the frame before the contact.
  The receive's guess-walk push changes that stick there: ×1.5 (body) or ×2, then cut to unit length.
- Play's aim read the stick stored at the press (`ai_press`), so a pushed return aimed with the stick from before the push.
- The fix is in `doubles_ai::run`: when the routine held its stick out this frame (`holds_out_stick`: swing 1, swing
  state), that stick replaces the press's. The contact reads it the next frame (`ai_contact_stick`, then `ai_stick`'s
  1/127 steps, then the singles kind lock, as before). A routine that pushed nothing holds out the same stick it pressed
  with, so nothing else changes.
- No decompile was read. The push was already ported (`hst_sim::rally`, checked against `ai_rally_*.bin`). Only the
  hand-over was wrong.

## Looking in the recordings
- The probes (`aim_probe.rs`: two ignored tests to drop into `crates/hst-play/src/play/`; `scan.py`, `airl2.py`, `land.py`)
  were in this entry's `scripts/`, which isn't in git (journals are .md only). What each one did is described below.
- `aim_probe.rs` `m189_probe` tries every whole-step stick (−127..127 on each axis, /127) with every contact kind
  (branch, kind, sweet or not, body, rising, the incoming slice, the deep button) through `shot::aim`. It looks for the
  stick that gives each recorded rally aim (player +0x3e90) bit for bit.
  - `match_s05.bin`: 168 aims. The only ones with no stick are the serves (made from the baseline at ±12.25), plus one
    return at vsync 23161 (player 3, from (−4.23, −10.17)). That return is probably a contact kind the probe doesn't
    try (e.g. a dive or an offset smash).
  - `eu-bot-match.bin`: 232 aims. Every one that isn't a serve has a stick.
  - Almost all have exactly one stick. That backs up M187's 1/127 steps for every rally contact, not just the two returns.
  - Longest stick: 0.9963 (43, 119). **No recorded contact has a whole-step stick longer than 1**, so the
    over-long case (both axes ≈0.7071 → 90/127, length 1.0022) isn't in either match.
- The pushed contact:
  - `ai_rally_s05_long.bin` records slot 5's receive calls from vsync 7593 to 32607, nearly all of `match_s05.bin`.
    The push is never set in it. So the US match has no pushed contact. The EU match has no AI recording to tell.
  - `airl2.py` over every AIRL recording: pushes at swing 1 show up only in the slowed and singles ones (`s05_slow2`
    3, `singles_base` 2, `singles_spec` 1, `singles_vpad_let_go` 1, `singles_vpad_wide` 1). Their out stick is the
    pushed one.
  - None of those records the aim (+0x3e90). But they do record the predicted path the next frames, which gives the
    landing. `m189_push_aims` aims slow2's three pushed returns both ways (topspin, sweet, no body: the contact's kind
    isn't recorded):

    | vsync, player | pressed stick's aim | pushed stick's aim | path's landing |
    |---|---|---|---|
    | 21919, p3 | (1.555, 3.746) | (2.364, 3.000) | (2.719, 3.511) |
    | 23436, p1 | (−3.112, 8.034) | (−4.854, 8.437) | (−5.384, 9.943) |
    | 26389, p2 | (2.734, 6.925) | (4.589, 6.701) | (4.082, 5.654) |

    Every landing is on the pushed stick's side, about 0.5 m from its aim sideways. The pressed stick's aim is 1.2–2.3 m
    off. The rest is the launch's timing scatter and margins.

## Proof
- `play::fifty_hz` (4 tests) passes unchanged. Neither bot match's returns are pushed, so both stay bit for bit as in
  M187.

## Not verified / not 1:1
- **The pushed contact, bit for bit:** no recording has a pushed contact together with the game's aim. The US match has
  no push (`ai_rally_s05_long.bin`). The EU match can't be told (no AI recording). The slowed slot-5 recording that has
  pushes doesn't record +0x3e90. The landings above show the pushed stick is the one aimed with, but only to about a
  metre, because the scatter isn't replayed. A bit-for-bit check needs a new capture of a slowed bot match (speed ×0.25,
  as `record_ai_rally.py`'s 6th argument) with +0x3e90 recorded. Slot 5 doesn't load in PCSX2 v2.9.179 (M185), so
  that needs slot 5 re-created by the human.
- **A whole-step stick longer than 1:** play passes it on uncut, as `shot::aim` (the ported aim) reads the pad. None of
  the ~330 recorded rally contacts has one (longest 0.9963), so whether the game cuts it is still unknown.
- **No unit test for the hand-over itself:** setting up a pushed receive in play takes a slowed match. The bot-match
  tests only show that nothing else changed.
- The US return at vsync 23161 matches no stick in the probe (see above). Not followed up: the bot matches part long
  before it.

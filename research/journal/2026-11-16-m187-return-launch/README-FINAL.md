# M187: the return's launch in both bot matches

## What it was
- The aim. The computer player's stick reaches the aim the way the game reads the pad: each axis in steps of 1/127,
  rounded to the nearest. The serve already worked this way (`ai_stick`). Rally strokes passed the AI's float stick
  straight through.
- slot 5's return (player 1, contact at recorded 298 = vsync 7864):
  - The receive routine's stick in play is (0.40947402, 0.29996747). As pad bytes that is 52 and 38 from centre,
    which read back as (0.409449, 0.299213).
  - `fixtures/match_s05.bin` records player +0x3e90, the game's aim result: (2.2517228, 0, −4.6330729). Play had
    (2.253573, 0, −4.6272726).
  - The swing-start point (+0x3ef0/+0x3ef8) was the same in both: (−3.0927196, 11.776895).
  - Working the aim backwards from the game's result gives a stick of about (0.40945, 0.29923), which is the
    quantised one.
- How it was found:
  - `fixtures/flights_s05.csv` shot 2 is this same return (its frame 3 is the recording's tick 301).
  - Play's spin (4.472406) and curve frames (51) matched the game's. Its side axis and launch frame were about
    2e-4 rad off, so the hit→target direction was wrong. The recorded +0x3e90 then showed the aim itself was off.
- The return of serve doesn't go through the pair chooser (`ai_aim_s05.bin` has no record at its press). It comes
  from the receive routine's zone aim, which reads no positions. So the routine's own stick was right, and only the
  hand-over to the aim was wrong.
- No decompile was read: the recordings showed the cause, as in M186. So this is a fix, not a clean-room port.
- The fix is in `ai_contact_stick`: every rally stick a computer player holds at its contact goes through
  `ai_stick`, the singles one after its kind lock.

## After the fix
| | US (slot 5, 60 Hz) | EU (M177, 50 Hz) |
|---|---|---|
| the return | bit for bit 299–370 | bit for bit 231–303 |
| the ball parts at | 371, the server's side's stroke (players had parted at 318, M185) | 304, the server's side's stroke |
| players part at | 291 (the 4e-6 slide, M188), then 318 (M185) | 250 (was 249): the server's partner (noted on M185: maybe its cause) |

## Proof (`play::fifty_hz`)
- `the_european_first_serve_set_up_plays_as_recorded`:
  - every player bit for bit through 249;
  - `return_flight`: the returned ball bit for bit 231–303.
  - Without the fix it fails at 231.
- `the_us_first_serve_set_up_plays_as_recorded`: `return_flight` 299–370. Without the fix it fails at 299.
- The whole hst-play suite passes (262 tests). `snapshot::a_bot_taking_over_plays_on` still holds with the changed
  bot rallies.

## Not verified / not 1:1
- **The push** (stick ×1.5 or ×2 at swing 1, then cut to unit length): the receive and rally routines change the
  stick the AI holds at its contact. Play's aim reads the stick stored at the press, before the push. Neither return
  here is pushed, so this wasn't checked (M189).
- **A quantised stick longer than 1** (both axes ≈0.7071 become 90/127, length 1.0022): play passes it uncut, as
  the serve's quantised stick. Whether the game cuts it to unit length isn't known: no recorded contact has one
  (M189).
- **Singles**: the quantising comes after the singles kind lock, by the same rule. It wasn't compared against a
  singles recording.
- **Later strokes** in both matches: not reached. The players part first (M185, noted there for the EU match too).
- **Live capture**: slot 5 still can't be loaded in PCSX2 v2.9.179 (M185), so everything here comes from the
  existing recordings.

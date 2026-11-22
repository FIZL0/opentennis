# B759: Steam Deck preset when docked

## What's built (branch `wip/B759`, not yet on the task branch)

- `Graphics::STEAM_DECK_DOCKED`: Default (the Deck's own preset) at a lower render scale. 67 % for now, a placeholder
  until real Deck numbers come in.
- `Graphics::docked(w, h)`: when the window is bigger than 1280×800 in physical pixels, the Deck's preset becomes the
  docked one, and at 1280×800 or smaller it goes back. Character Shading is kept as the player set it: the user's Deck
  had it off, so an exact preset match would have missed their case. A preset or setting the player picked
  themselves stays as it is.
- System `deck_dock` (only when `SteamDeck=1`): runs only when the window size changes, so a preset picked while
  docked stays until the next dock or undock. It edits the `Graphics` resource in whichever process is running,
  match or menu. The match gets the Deck preset as flags, then switches when it first sees its window.
- Test `deck_docked` (passes, `tools/check.sh -p hst-base graphics`). Release-note line on the WIP branch.

## Igpu bench, 1920×1080, 4 bots doubles, match ends at 25 s (ceremony + result page after), immediate present

`context/b759/run1`, `run2`. fps per 5 s window:

| settings | rally (5-25 s) | ceremony/page (25-45 s) |
|---|---|---|
| 100 % 4x High | 45-54 | 45-50 |
| 100 % 4x Low | 51-57 | 45-52 |
| 85 % 4x High | 47-64 | 55-60 |
| 85 % 2x High | 55-67 | 54-60 |
| 85 % 2x Low | 67-74 | 62-68 |
| 75 % 4x High | 52-80 | 61-71 |
| 75 % 4x Low | 70-88 | 71-78 |
| 67 % 4x High | 77-91 | 77-82 |
| 67 % 2x High | 74-92 | 77-82 |
| 1280×800, 100 % 4x High (handheld) | 83-102 | 60-95 |

Render scale costs the most. MSAA 2x vs 4x made no difference at 67 %, and Low shadows bought less than one scale
step. Some windows (e.g. 100 % 2x: 33) were clearly disturbed by other agents' GPU use.

## Held: real Deck data

The run's owner (2026-11-21) measured the real Deck docked (nested gamescope 1920×1080, immediate, 4 CPUs doubles,
mod court, 100 %/4x/High): rallies run about 95 fps, GPU about 75 % busy. The igpu stand-in (46 fps at the same
settings) is about 2x too pessimistic, so docked quality must not be cut on its numbers alone. They are measuring
match end on the Deck next. To pick the settings, take the real match-end fps:
- If 100 % holds 60 with headroom at match end, docked may not need a scale cut at all. Then a small step (85 %),
  or no docked preset, is the call.
- Otherwise take the smallest cut (by the scale ladder above: 85, then 75) whose Deck match-end low stays well above
  60.

## Not verified / not 1:1

- The switch has not run live under `SteamDeck=1`: the check was skipped so it wouldn't disturb other agents' GPU
  benches. Only the unit test covers it.
- The settings are not picked (see above): the igpu numbers overstate the Deck's cost.
- Bench runs used 4 bots, not 4 humans (the bench's autoplay); the Deck read was 4 humans.
- Rendering only: nothing here reaches the sim.

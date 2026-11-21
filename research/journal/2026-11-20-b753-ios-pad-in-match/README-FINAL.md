# B753 — iPhone: B752's fix checked in a match

Release build of main at 598ab3a (B752 merged; `tools/ios-ipa.sh`), installed on the iPhone 15 Pro with iFizBox's
`tools/ui-15.sh` (under its lock; waited ~35 min while F330 and then R49 held it). Taps went through WebDriverAgent's
`/actions` from a small client that refuses to act unless B753 holds the lock. Screenshots and the app's
`Documents/hst-log.txt` are in `context/b753/`.

## Sequence (portrait)

1. Launch → **one tap on "Tap to start"** (the failing case in B752) → main menu, on-screen pad shown.
2. Pad ✕ → Local → character select, 1P "Controller 1"; ✕ ✕ picks Random for 1P and the COM; ✕ on "Ready to play?"
   starts the match. Log: `starting the match: … --pads 0,-`, so 1P is the on-screen pad (gamepad 0).
3. In the match (Wild Green court):
   - **Stick**: held left for 1.2 s, the player walked left along the baseline (the camera-followed court slid
     right by about a quarter of the screen). Log: `takes the stick` / `lets its control go`.
   - **✕**: tossed and hit the serve; in the rally 1P lunged at the COM's return (missed: "Return Ace").
   - **○**: served (slice). **△**: during the rally 1P dove for the return. Log: `presses button 3` for each △.
   - **Pause** pill: the pause menu opened; the stick moved its cursor down to Settings and back up, and ✕
     resumed the match.

So after a splash tap, the on-screen stick and buttons move and swing 1P in a match, and the pad drives the in-match
pause menu. No code change needed.

## Not verified / not 1:1

- No rally groundstroke actually hit the ball back: blind timed taps from the desktop (each WDA action plus a
  screenshot takes about a second) kept missing the COM's returns. The swings themselves did happen on the ball,
  and the hit/miss timing is the sim's own, not the touch path.
- Only the portrait layout was driven; landscape uses the same touch path with a different layout.
- The browser's touch pad (`web/touch.js`) was not tried in a match (no phone browser run).
- Front-end/touch behaviour is ours: nothing to match in the original.

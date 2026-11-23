# B765 1.9.1 fixes

- Stats board values: back to the stats sheet's own white 16×24 digits (B764 had moved them to the result board's
  orange outlined digits); the score board keeps those.
- Graphics preset "Default" shown as "Medium" (the preset is derived from the settings, never saved by name).
- Touch stick: the faintest haptic (iOS selection tick, Android CLOCK_TICK) each time the stick, past the game's dead
  zone (48/127), crosses into another of eight 45° sectors; in a match only, under the Vibration setting.
- Hidden □ in a match: no longer a control (no press, no haptic).
- Pill seams (user's iPhone shot: end screen's Rematch pills, Longest Rally bar): the end screen and popup renderers
  placed each slice in percent without pixel snapping, so a shared edge could round apart. Both now snap every
  quad's edges to whole physical pixels of their parent (`widescreen::put_snapped`), as the main menu already did.
- Website: iOS links go to the FIZL0 apps page (https://fizl0.github.io/apps/); web/ios.html removed.

## Not verified / not 1:1
- Haptics and the seam fix not checked on a phone here (no device; iOS/Android compile only in the release build).
- The notch uses a round dead zone, the game's run direction a per-axis dead square: a feel cue, not gameplay.

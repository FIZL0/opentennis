# B705: the zoom-out into the Game, Set, Match screen

User: after the winners' celebration at the match's end, the cut/zoom into the Game Set Match screen doesn't look
right (not the change of ends, B703).

## The original (PCSX2 copy 2, bot doubles from slot 5, games poked to match point)
`context/b705/b705_end.py` (scratch; the journal keeps only markdown) plays to the match end, saves scratch slot 8 at the ceremony's start and takes a few F8 shots
(`context/b705/orig1`); `context/b705/b705_cam.py` loads slot 8 and logs the view matrix and fov every vsync for 1100
ticks (`context/b705/cam1.txt`, no missed frames). Only RAM was watched, no decompile read.

- Ceremony (265 ticks here): the clip's eye stays put ~3 m in front of the mark while its fov widens 30° → 75°
  (ticks 60–260): **the zoom-out is the original's own clip**, and our port already plays it the same.
- On the result page's first tick the original **cuts** (under the page's 60-tick dissolve) to another camera:
  70° fov, eye 1 m up, 2.0575 m from the shown side's mark (0, −6.4), looking level at it, starting on the
  ceremony camera's side, then circling the mark exactly π/720 rad a tick (eye x falling first) for as long as the
  page is up. At +586 ticks it looks at the players' backs (`orig1/c0966_p586.png`).
- Our port held the clip's last, widest view behind the page for good (P26c/B38 had noted this as a gap): the
  zoom-out just stopped and froze, which is what looked off.

## Fix
`ceremony::orbit`: once the clip has ended and the page is open (the ceremony's tick counter goes past the clip's
length only once the page's loop ticks it; while the page waits for its still the clip's last frame holds, so the
dissolve still is the wide view, as in the original), the view is that circling camera: offset (the shown mark),
turned π at the far end like the clip. Test `ceremony::tests::page_orbit` checks the recorded first-tick and
quarter-turn eye/forward values and the far-end mirror.

Checked: `tools/check.sh -p hst-play ceremony` (8 pass), clippy, audit; release-build shots
(`context/b705/port1/sheet.jpg`, HST_STATS end, court 10): clip end wide → page cuts to a close chest-height view
of both players → ~7 s later seen from their side, like the original's page.

## Not verified / not 1:1
- Only a near-end bot doubles win was recorded. The far end (we turn the orbit π like the clip), singles, and the
  lost clip (whose camera has a −1.3 height offset) weren't recorded: the orbit's centre/height there is assumed the
  same (mark, 1 m), not seen. Its fixed 1 m height ignores the clip's height offsets.
- Radius 2.0575, height 1 m, fov 70° and the step are read off the recorded view matrix (to ~1e-4), not from the
  game's code; by-eye presentation (in-match presentation rule).
- The orbit uses std `sin`/`cos` (presentation only, never the sim). The 3-tick-early start of the page's dissolve
  noted in B38 is unchanged.

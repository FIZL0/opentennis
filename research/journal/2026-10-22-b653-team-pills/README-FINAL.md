# B653 — A Team / B Team in our own pills and lettering

## What shows the teams in a match
Only two places draw the teams: the score panel's doubles banners before each serve (`panel.rs`) and the doubles
winners' ceremony plate (`match_stats/ceremony.rs`). The game/set result board has per-player plates only (pill,
face, slot label), with no team plate. The ceremony already used our pill and the menus' rebuilt `inpane_team`
words (f1d56f2b). The panel still drew `inpane_team1` from the disc: its blurry half-pill pieces and pixel words. That
is the "A Team" banner sitting over the COM plate in the user's screenshot.

## Change
`panel.rs`: each team banner is now `menu_art::pill_quads` on our pill sheet (`pill_layers`): a dark rim, a white
ring, the team colour on top (the same three layers and colours as the ceremony). The pill is 128×24 with its flat
end run 16 px past the screen's edge, so what shows is the original's 112-px tab, round on the inside. The words are
the rebuilt `inpane_team` cells (96×32) at ¾ size, 72×24, over the middle of the tab. The panel loads `inpane_team`
in place of `inpane_team1`, and puts the pill sheet right after its disc sheets (`FACES` moves up one).
Placement goes through the panel's existing `widescreen::pin`/`vplace` per side, at uniform scale.

## Verified
- `panel::tests::doubles_pieces`: both banners' pill ends (left one off the left edge, right one off the right),
  the tint layer, and the words' cells and rects.
- `--shot` doubles all-CPU at 1280×720 and 1600×720 (`context/b653/run1/a.png`, `wide.png`): both banners are
  sharp pills with "A Team" (red) / "B Team" (blue), pinned to their sides with the panel, same shape at both
  aspects.

## Not verified / not 1:1
- The banners are now our art, not the disc's (by request), so there's nothing to match pixel for pixel. Their position, size and
  colours are the original banner's.
- No shot of the "Score to win" line hiding a banner. The test covers that the banner goes, and that logic is
  unchanged.
- The result board has no team plates to change. I didn't add any, because the original's board shows none.

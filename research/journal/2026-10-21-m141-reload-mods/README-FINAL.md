# M141 — Reload Mods button after the transfer queue ends

Built on M136 (pill + end-of-batch Reload popup). M136 wasn't in main when this ran (the runner was merging it);
its code commit (587a6689) was cherry-picked as the branch's first commit, code only (no plan/journal/notes), so
it merges as identical changes once M136 lands.

## What changed
- `View::idle()` (transfers.rs): nothing queued or downloading, no upload queued or going.
- Reload Mods button in the pill's place (`transfers::reload_button`, `Act::Reload`): mods changed since start,
  idle, the popup down and a calm screen (M136's `calm`). A tap reloads (phone apps: "Close to load mods", closes).
- Mods grid's footer Reload (B387's) and the main menu's Mods-row "Reload" note now wait for idle too;
  `hold_cursor` keeps the grid cursor on the same tile as the footer word comes and goes.
- Never asks mid-queue: leaving the Mods page while transfers go no longer asks at once; it marks the popup due and
  `offer` (now waiting for downloads as well as uploads) brings it up when everything's done.
- Play local / Play online with mods changed, idle, and no "Play without them" since the last change: the popup asks
  "Reload now / Play without them"; Play without them carries on with the same pick (re-pressing ✕, so Online's
  update prompt etc. still follow). Asked once per change of mods.
- Shots: `HST_TRANSFERS=done` (button), `HST_TRANSFERS=play` (Play ask); checked in context/m141/run2/.

## Tests
transfers.rs: `the_reload_button_waits_for_the_queue`, `the_footers_reload_waits_for_the_queue`,
`leaving_mid_download_asks_at_the_end`, `play_asks_again`.

## Not verified / not 1:1
- Front-end only (ours to design, PLAN.md); no original to compare against.
- The top Reload Mods button is mouse/touch only; pads reach Reload through the Mods page's footer pill, the main
  menu's popup and the Play ask.
- The button isn't shown on the select screens, lobbies or in a match (a restart there would drop picks/the room);
  the pill itself still shows there while transfers run.
- Not driven with a real store download/upload end to end (unit tests + sample shots only); B601 covers the live
  pill check.

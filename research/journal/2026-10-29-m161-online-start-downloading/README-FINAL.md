# M161 — Online Start waits for downloads

Front-end/lobby work (ours to design, no original to match).

## What changed
- `Room::held` (net.rs): the host's start is held while `Room::fetching` (any peer downloading mod / court looks,
  M122) is non-empty. The host's Start, or everyone being ready (B215 auto start), goes to the hold instead of the
  countdown when someone is downloading; once nobody is, the countdown starts by itself. A download that begins during
  the countdown (e.g. a repick to a mod) stops it and holds again. A player leaving drops the hold (like the countdown).
- Cancel (host's Start / guest's Ready, `Cmd::Cancel` / `Msg::Cancel`) now stops the countdown, the hold, or the
  B281 "choose whether to download" wait after the countdown (not once the start's round-trip probe is out).
- Lobby Start button: "Downloading..." while anyone downloads (pressing it holds the start); held it stays
  "Downloading..." with "Starts once everyone has it: <who (%)>" and the legend's OK is Cancel.
- Tests: `net::tests::downloads_hold_the_start` (local host + guest), `a_held_start_is_cancelled_from_start_and_ready`.

## Not verified / not 1:1
- Online lobby is our own front-end (no original equivalent), so nothing to compare to the game.
- Not tried by hand in a live two-machine lobby; covered by the local-connection test and the menu test only.

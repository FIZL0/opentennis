# M197: online lobby resumes an unfinished match

## What it does

- **Save on the way back.** A peer drops and the host goes back to the lobby. Before leaving the match, the host captures
  a match save (online mode, the room's flags). It writes the save to the user's saves and hands it, with the line-up,
  to the lobby.
- **Offer.** In the lobby the host offers the save as `Room::resume`, which reaches every guest:
  - `chip`: "Sets · games · points · NAME to serve";
  - `score`: the set's games;
  - `on`;
  - the saved room.
- **Identity.** Every machine sends a persistent player id in its Hello:
  - native: `player.txt` beside settings;
  - browser: the tab's sessionStorage.

  The host keeps a peer → id map, host-side only (`#[serde(skip)]`). When a newcomer has the id of a saved player who
  is away, the saved room renames the old peer to the new one. That puts them back in their own seat with their own
  pick.
- **Resume on.**
  - The save's doubles, court, court mod, umpire, sets, games, mods and markers become the room's, with its CPU seats
    and its players still present in their own seats.
  - Seats of players not back are held: newcomers can't take them and joins count around them.
  - Picks, team, venue and bot changes are ignored, from the host's menu and from guests.
  - Start and the auto-start wait for `resume_ready` (every saved player back in their seat).
- **New match.** Turns resume off: the save's CPU seats open again and the host's own court and rules are sent again.
  The picker can switch back to Resume while nobody new has sat down.
- **Starting.** Resumed matches start held on every machine (`Recover.hold`). Once all have loaded, the host sends the
  restart through the existing Msg::Restart path with tick `u32::MAX`, its state being the save decoded and re-encoded.
  Every machine decodes it and resets to the point's start (`save_game::at_point_start`, shared with offline resume).
  The intro is skipped.
- **Menu.**
  - A gold-ringed chip "Unfinished match · …" sits beside the title. When the top bar has no room, the chip's words
    move into the Start/Ready button's lower line, which is full-width on those windows.
  - A MATCH picker (Resume 3-5 / New match) leads the bottom bar, carved from Start's row or from the strip's left.
  - Locked cells are dimmed and lose their ‹ ›.
  - A held card reads "Waiting for NAME / Rejoining with code XXXX takes this seat back".
  - Players who are back get a "From the save · Back" tab.
  - Start reads "Resume / n of N back · waiting for NAME" and starts only when everyone is back.
  - Shots: `HST_MENU=lobby-resume`, `lobby-resume-new`. Checked at 1280×720, 900×720, 1600×600 and 700×1000.

## Tests

- hst-base `a_resumed_lobby_keeps_the_dropped_seat`: held seat, refusal of a stranger, rejoin by id, start gate.
- hst-play `a_dropped_match_resumes_from_the_lobby` (HST_NETPLAY): a real drop, the lobby save, then a fresh host and
  the returning guest. Both machines reach the same state hash as the save decoded alone. Score kept, play goes on.
  Mutation (no decode of the save) fails it.
- hst `a_saved_match_is_resumed_or_not` (locked presses, Start waits, Match picker), `lays_out_at_any_aspect` with a
  resume room (host and guest).

## Not verified / not 1:1

- No PCSX2 comparison: online lobbies and resuming are ours. The original has no online play.
- No live two-machine run over the real relay. The netplay test drives the real host/guest code in one process.
- The resumed match restarts from the start of the point at the drop, not the exact rally moment (the offline resume's
  rule).
- The mockup's look is approximated, not copied. Menus don't need to be 1:1:
  - the held card is a dark panel inside the dashed card, not a greyed face;
  - "Back" is a green tab on the card's top edge.
- Two native copies on one machine share `player.txt`. That's harmless: only a saved player who is away can be
  matched, and the host's own peer is "host".
- A browser tab that is closed (not reloaded) loses its sessionStorage id, so it can't take its seat back. The host can
  still pick New match.
- Mods setting on New match: the save's value stays (it was the host's own in the dropped match). Court, umpire, sets,
  games and markers go back to the host's menu values.

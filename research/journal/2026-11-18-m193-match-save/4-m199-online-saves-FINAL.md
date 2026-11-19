# M199: online match saves, hosted again from Continue

M197 already resumed a dropped match from the lobby it went back to (the save and line-up kept in memory). M199
adds the rest of the online save's life: a save made by the host's Save and Quit, or the lobby's auto-save, can be
hosted again later from Continue.

## What changed

- **The save keeps its lobby.** `save_game::capture` in online mode adds `Setup::lobby` (`save_game::Lobby`): the
  match's lobby as it played (seats with picks, court, rules, names) and each guest's player id by peer. The ids
  live only in the host's memory (a lobby never sends them); the save is the host's own file. Live-only things
  (pings, places on the select, an earlier offer) are left out. Saves from before this read with none.
- **Save and Quit online** already saved `Mode::Online` (M194). It now also saves the lobby. Its box says "Host it
  again from Continue;".
- **Continue → Open lobby** (`continue_screen::resume`): the save is read, and `netplay::host_again` puts it with its
  lobby into `netplay::SAVED`, the same slot the drop path uses. It goes to Online with Host chosen and sets
  `View::host_save`. A save without a lobby shows "Saved without its lobby" (or "Can't read it") and stays.
  Entering Online any other way clears `host_save`. So does choosing Quickplay, Find or a code.
- **Host**: with `host_save`, Host skips the select and goes straight to the lobby, since the picks are the save's.
  Once the lobby's room is open, `poll` sends `Cmd::Resume(netplay::saved_offer())` once, and the cursor goes to the
  MATCH picker.
- From there it is M197's lobby:
  - the picker (Resume 3-5 / New match) at the head of the bottom bar;
  - court and rules from the save, locked;
  - "From the save · Back" and "Waiting for X / Rejoining with code … takes this seat back" cards;
  - returning players matched by id;
  - Start waits for everyone.
  At launch `resume_from_save` turns the save into `RESUME`. Every peer sets up from the save's line-up, and the
  host's restart sends the decoded save (`snapshot::decode`).

## Tests

- hst-play `an_online_save_keeps_its_lobby`: the lobby and its ids come back from the file without the live-only
  fields, and an older head reads with none.
- hst-play `a_dropped_match_resumes_from_the_lobby` (HST_NETPLAY) now resumes from the save **as read back from its
  bytes**, its lobby taken from the file. Both machines reach the save's state and play on in step.
- hst `continue_screen_steps`:
  - an unreadable online save is refused;
  - a written one sets up the offer (score, on, the guest's id);
  - Host goes straight to the lobby with `Cmd::Host`.
- `HST_NETPLAY=1 check.sh -p hst-play netplay`, `-p hst online`, `-p hst menu::`, `-p hst continue_screen`: passed.

## Not verified / not 1:1

- No PCSX2 comparison: the original has no saves or online play; menus are ours (PLAN: front-end).
- No live two-machine run over the relay: there is no second machine in an unattended run. The resume itself is the
  in-process netplay test. The fresh lobby's `poll` → `Cmd::Resume` step is not driven end to end. It uses the
  `Cmd::Resume` path that M197's lobby tests cover.
- The held card keeps M197's wording, "Waiting for X / Rejoining with code … takes this seat back", not the task
  line's "Seat kept · Waiting for X". That follows the approved concept (CONCEPT.md *Online lobby*).
  `concept.html` isn't in the checkout.
- Guests rejoin by player id. Ids are per browser tab (sessionStorage) or per install (`player.txt`), so a guest on
  another device or a closed tab can't take their seat back; the host can pick New match.
- The lobby's code is new each time it's hosted, not the dropped match's code.
- Online saves made before M199 (M197's auto-save) kept no lobby, so they can't be hosted again; they can be deleted.

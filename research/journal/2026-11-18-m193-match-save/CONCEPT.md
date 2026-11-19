# Match save & resume: approved concept (user 2026-11-18)

Mockups: https://claude.ai/artifact/BbnMCMW5SnKWdeeYKCqnGG (the user's; this file is the spec the tasks build
from). Tasks: M193 core, M194 pause, M195 main menu + Continue, M196 match end, M197 online lobby.

## Rules

- A save is the match setup (seats, characters, costumes, hands, CPU strength, court, umpire, sets/games, mods with
  versions) plus the full match state from the **start of the current point** (snapshot encoder, RNG included),
  the game build and the mode. Resuming opens on that serve; play goes on exactly as it would have.
- One save per line-up + mode (a new one replaces it); the newest 5 are kept.
- Won't load: another build → "Saved in v1.7.3"; a mod updated or missing → "Mods changed". Delete always works.
- Addition only: a match never saved plays exactly as before; replay tests unchanged.
- Online: the host keeps the save, made automatically on "back to lobby" after a drop, or by the host's Save and
  Quit. Players matched by identity, not name. Guests need no copy: Start sends the state as a restart does.

## Pause (M194)

Unchanged look: brown dialog, peach chosen pill with the hand cursor, "Paused · N sets N games" bar, court and
umpire top right, scoreboard below, key legend. Rows: Resume the Match, Settings, Controllers, **Save and Quit**,
Return to the Menu. Save and Quit → same-style box "Match saved" / "Carol 6-4 4-2, 30-15. Continue it from the
main menu; it picks up at the start of this point." / OK. "Leave the match?" adds "It won't be saved. Save and
Quit keeps it." (Yes / No, No chosen). Online: only the host gets Save and Quit.

## Match end (M196)

After the Game, Set, Match ceremony: the stats page on the right side of the screen (always shown, no Stats
button), on the left three pills in the pause dialog's style: **Rematch**, **New Match** (character select with
picks kept; court and rules are changed there), **Return to the Menu**, with a one-line note under them for the
chosen one. Online: Rematch (everyone readies) and Back to Lobby.

## Main menu and Continue (M195)

- With a save, a tall **Continue** card heads the list: "Continue" / "Carol vs Will · 6-4 4-2 · 30-15". Its
  preview: faces either side of a navy scoreboard (sets, serve dot, points), a Ready chip, "Picks up on Carol's
  serve at 30-15", the description, facts MODE / COURT / SAVED. Rematch shrinks to a slim card below it.
- One save: Continue resumes it. Several: the **Continue** screen (gold "< Continue" tab, "3 saved · newest
  first · keeps 5"): one card per save, styled like the lobby's player cards: header in the mode colour
  ("LOCAL · SINGLES" red, "ONLINE · DOUBLES" blue, grey when it can't load) with a chip (Ready / In its lobby /
  Mods changed), faces, names, mini board, court and date at the foot. Under them the navy bar (COURT, UMPIRE,
  LENGTH, SERVING of the chosen save) and the yellow start button: "Resume ▶ / from 30-15, Carol to serve",
  "Open lobby ▶ / host it again, code K7QX", or greyed "Can't resume / <reason>". Legend: Move, Resume, Delete
  (Del/L), Back.

## Online lobby (M197)

Lobby as now, plus: a chip by the title "Unfinished match · 3-5 · 40-40 · Sunny Day to serve"; a **MATCH**
picker leading the bottom bar (Resume 3-5 ⇄ New match). On Resume, court/umpire/sets/games come from the save
and show locked (dimmed); the dropped player's card is dashed with a greyed face, "Waiting for ACE-42" and
"Rejoining with code K7QX takes this seat back"; the others say "From the save · Back". Start: "Resume ▶ / 3 of
4 back · waiting for ACE-42". On New match the chip goes, the settings unlock and Start is the usual one.

## Open questions

none

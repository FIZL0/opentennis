# M145: Installed mods filter

Most of what the task asks for was already on Mods → Installed when this task started (shot of the page at the start,
`context/m145/before.png`):
- kind tabs Characters / Courts / Umpires / Texture packs, each with its count (B373/B383/M8f9/M5b), switched with
  L2 / R2 on a pad and a tap on touch;
- the name search box (name, uploader, game), typed on the keyboard or with the on-screen keyboard from a pad (B383/B441);
- the Game filter and sort pills on the character, court and umpire tabs;
- layout for any aspect ratio via `grid::lay`, with tests covering 4:3 to ultrawide.

The search already applied on Installed and stayed set across a kind switch. The missing piece: the kind tabs' counts
ignored the search, so with a name typed you couldn't see which kind had hits, and an empty tab only said "No mod matches".

Changes:
- `View::kind_counts`: on Installed, Browse and My uploads each tab counts the mods the search finds on it (Upload's
  list isn't searched, so it keeps the plain counts). Tabs never hide because of a search (`kinds_shown` still uses the
  totals), so a tab showing 0 is still there to switch to.
- `grid::View::searched` / `found_elsewhere`: when the current tab has no match but other kinds do, the empty grid says
  "No umpires match. Other kinds do: Characters 1, Courts 1, Texture packs 1."
- Test `installed_search_and_kinds` (mods_screen.rs): 41 characters, 2 courts, an umpire and 3 texture packs. It types
  a search, checks each tab's count, keeps the search across R2, checks the empty tab's words, taps the Texture packs
  tab on its drawn rect, checks that the pack list is searched and stays in its order, then clears the search.

## Not verified / not 1:1
- The original game has no mods page, so there is nothing to compare against.
- No screenshot of the empty-tab message: `--shot` runs can't type into the search (no `HST_MENU` sample sets a
  search). The test checks the message's words. It's drawn by the same 4-line paragraph as the existing messages,
  in the same grid rect, so it fits like they do at any aspect.

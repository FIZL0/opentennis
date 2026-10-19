# B573 — bot rank on character select

The full (local) character-select card of a computer player said only "COM" under its character; the online
compact card and the confirm plates already showed the rank. `roster.rs` `com_line` now gives the band's second
line "COM - Lv 3" etc.: the AIParam rank for the character (a mod's donor) and costume it has now, in this
match's singles/doubles (`com_rank`, same as the confirm plate). On Random it stays "COM" (no character yet).

Checked: `a_com_card_shows_its_rank` unit test; `HST_MENU=coms` shot (context/B573/coms.png) shows
"COM - Lv 5" on 2P's Ashley card.

## Not verified / not 1:1
- Front-end layout is ours (PLAN: menus don't have to match); the original shows the rank as a "Tennis Lv" pill on
  its confirm screen, here it's the card header's second line. No PCSX2 comparison needed.

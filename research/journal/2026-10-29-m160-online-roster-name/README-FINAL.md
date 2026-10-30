# M160 — online roster name defaults to the online name

`crates/hst/src/play/main_menu/names.rs`: a seat's name now goes through `own_name(dev, online)`: the name picked
for the device (△ picker) wins; else, in an online lobby (or the online match's `--names`), this machine's first
local player takes the machine's online name (Online → Name, `net::Online::name`), cleaned like any tag (6 chars,
menu-font characters). While the online name is still the default "Player" (any case) or blank, nothing changes:
the usual 1P–4P. The machine's other local players keep 1P–4P (one online name per machine). Remote machines see it
through the live pick's `player` field as before, so the lobby band, select card, over-head tags and result screen
all follow.

Test: `names::tests::online_the_first_player_takes_the_online_name`.

## Not verified / not 1:1
- Front-end feature of ours (no original counterpart); not checked live in a two-machine lobby, only by unit test.
- Online names up to 16 characters are cut to the tag's 6 (e.g. ROBINSON → ROBINS).

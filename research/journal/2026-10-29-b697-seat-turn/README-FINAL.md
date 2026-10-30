# B697 — mod seats no longer line up with the sitting body

## Cause
Mod takes can sit on a seat of their own (`extras.seat`, 2026-10-26, `character::seats`): the seat model is spawned
as a child of the rig root, unturned. On 2026-10-27 (98bd3a48, then 5b7facaf) a mod's own post-point reaction
(0x2c..0x2f) started being drawn with the donor's whole turn applied to the `Bip01` root bone
(`CharacterData::reaction_turn`), so a seat stayed put while the body turned 30–55° off it. Anya's and Kratos's
seated takes are all `re_gu_set`/`re_di_set` (0x2e/0x2f), which are both the game-won/lost reactions and the
up/down emotes. f795259d (2026-10-28) took the turn off emotes (`OwnFacing`), but its system ran unordered against
`animate`, so an emote's first frame could still be drawn turned.

## Fix
- `character::seats` gives the seat the same turn `animate` gives the root (`seat_turn`: the reaction turn of the
  drawn clip, none with `OwnFacing`), kept while it pops out.
- `emote::own_facing` runs before `character::animate`.
Stock characters have no seats and no reaction turn: unchanged. Draw-side only, no sim/RNG touched.

## Test
`hst-base character::tests::seat_turns_with_the_reaction`: Anya's seated won-game take, seat rotation = the
reaction turn (≈53°) without `OwnFacing`, identity with it. Skips without the disc or `mods/hst-mods/oob_pc12_anya`.

## Not verified / not 1:1
- Not looked at on screen (unattended run, no shot of a seated reaction taken); the test checks the seat's
  transform equals the root's turn, which is the same frame the body is turned in.
- An emote whose player is released while its seated clip still holds the last frame drops `OwnFacing`, so body
  and seat turn together by the reaction turn at that moment (now aligned, but a visible turn); in practice the
  released player sets a new motion at once.
- Presentation only (mods); no comparison with the original applies.

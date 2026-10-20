# B599 — emotes sometimes play no voice

## Finding
Rendered every emote pick (the four emote programs 7–10, every key the emote cycle can reach) on every stock
character's voice banks (characters 0–13; singles and doubles sets; variants a/b; court 7's c/d) through the mixer.
All singles banks sound every take. The doubles banks (loaded with three or four players) only hold programs 7 and 8
keys 0–1 and programs 9 and 10 key 0. The emote cycle (B300/B500) used the singles key ranges whatever the player
count, so in doubles 7 of the 12 picks per character were silent (7:2, 8:2–4, 9:1, 10:1) — the same for every
character, Carol included. The character select plates use the singles bank: fine.

## Fix
`emote::bank_last(program, players)` caps a disc-bank emote cycle to the takes the loaded bank has; `line` gets the
match's player count. A mod's own takes (B596) are untouched. The key memory stays match state, the range depends
only on the player count (the same on every peer), no generator draws.

## Tests
- `every_emote_pick_speaks`: every character × bank × emote, cycled 12 times, rendered; lists any silent pick.
  Verified it fails (doubles only) with the cap removed. ~1.6 s.
- `doubles_emote_lines_keep_to_the_doubles_takes`: the doubles cycle order.

## Not verified / not 1:1
- Emotes are ours (not in the original), so there's nothing to compare against in PCSX2; checked by rendering the
  mixer output, not by ear in a live match.
- In doubles each emote has fewer takes (2, 2, 1, 1) than in singles: the doubles banks have no more. Loading the
  singles bank too for variety was not done (extra memory per player for an addition).
- Serving vs in play: the voice pick doesn't depend on the phase (the existing server/rally emote tests cover the
  emote itself), so it wasn't tested separately.

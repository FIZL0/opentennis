# B591 — emotes in the gap after a call

After a call (point, fault, let) the post-point time starts with a stretch where the human still moves, until the
players' reaction (`post.reacted`). B472 had closed the whole post-point time to new emotes; B591 reopens that gap.

## Change
`play/emote.rs` `tick`: a flick starts an emote unless the match is in the post-point time *from the reaction on*
(`reacting`: `Phase::Post` and the post-point already reacted). Unchanged: B363 (a point that wins a game/set closes
emotes in its whole post-point time), the reaction cuts any emote (B278, `react` in play.rs), changes of ends closed,
the server before its toss, `idle` (nothing under way). play.rs untouched.

## Tests
- `no_emote_between_points` reworked: a flick 3 ticks after the fault's call (before the reaction) emotes; the same
  with the post-point turned into a game/set win does not; the reaction ends it; a fresh flick two ticks into the
  reaction starts nothing through to the next serve; a flick in the second serve's wait emotes.
- All 11 emote tests pass.

## Not verified / not 1:1
- Emotes are ours (not in the original): nothing to compare against PCSX2. Not checked by eye in the app (unattended run).
- Only the fault's gap is driven in the test; a won point's and a let's gap go through the same `Phase::Post` /
  `reacted` gate, not exercised separately.

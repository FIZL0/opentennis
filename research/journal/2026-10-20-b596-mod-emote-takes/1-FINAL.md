# B596 — a mod's emotes speak all of its own takes

**Problem.** Emote lines cycled over the disc banks' keys (`emote::voice_keys`: 9/10 → 0–1, 7 → 0–2, 8 → 0–4), so a
mod with more takes (T-Bone: 9 has 4, 7 has 5) never voiced the rest; with fewer (T-Bone's 8 has 3) the wav index
wrapped and repeated a take back to back (keys 3, 4 → takes 0, 1).

**Fix.** A mod's take count per program (`mods::voice_takes`: `<program>_<key>.wav` files in its voice folder; a
costume pack none, it speaks its disc character) goes into the player's `CharacterData::voice_takes`
(`mod_match::load`), and `emote::line` cycles over `0..n` for a program the mod has takes of
(`emote::own_voice_keys`), the disc range otherwise. Online the count comes from the owner's lobby pick
(`mods_online::Wire::voices`, emote programs only, checked 1..=64), so every peer's key memory (match state, in the
snapshots) moves the same whether it has the takes or plays the stand-in. The select plates use the loaded mod
bank's own count.

**Tests.** `emote::a_mods_emote_lines_go_through_its_own_takes` (cycle over 4/5/3, disc range for a program it lacks),
`emote::a_mod_counts_its_own_takes` (folder count, costume pack none), `plate_emote::a_mods_emote_voices_take_turns`
(T-Bone's packaged mod on the plates), the wire round trip / check / stand-in in `mods_online`.

## Not verified / not 1:1
- Not heard by ear in a running match or on the plates (unattended run); the keys picked are tested, playback of a
  mod's wav by key was already covered.
- Remaster-only behaviour (mod characters), so nothing to compare against the original.
- A pick from an older build carries no `voices` (serde default: empty), so that mod's emotes cycle over the disc
  range on every peer, as the older build does itself: peers still agree. Not tried across builds.

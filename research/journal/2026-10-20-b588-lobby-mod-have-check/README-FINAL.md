# B588: online lobby asks to download mods both players have

## Cause
The lobby decides "this machine has that mod" in `mods_online::find`: a received copy, or an installed mod with the
same id **whose whole pack hashes the same** (the "looks" hash: every data file, byte for byte). Everything the
player sees decides by id alone (the seat's card in `align`, `borrow`, `remote_mod_name`), so:

- two installs of the same mod that differ in any file (another version: B584 just bumped every store mod; or a
  copy built locally, like the uploader's own `mods/` folder) hash differently → the download prompt comes up;
- after the download the select/lobby still draws this machine's own copy (matched by id) and the seat note is
  hidden (`remote_mod_name` is `None` for an id this machine has) → nothing visible changes.

Checked the hash itself is consistent: a Python copy of `pack`+FNV over store installs in
`~/.local/share/hst/mods` gives the store's published `looks` exactly (Alia, Alex, Allan), while the locally built
Phoebe (version edited by hand, zeroed store stamp) doesn't match the store's: the case that prompts.

## Fix
`find(w, costume, roster, cache)`: after the exact-hash match, fall back to an installed mod with the same id whose
picked costume has the pick's rig (same joint names and parent tree, read through the cached `kept`). Gameplay
already comes from the wire (`for_match` overwrites donor/hand/params/AI/skeleton/emote lengths), so only the
looks come from the local copy. A same-id mod on another rig still asks (its mesh wouldn't fit the wire's
skeleton). Test: `mods_online::tests::the_same_mod_in_another_version_is_found`.

## Not verified / not 1:1
- Not tried in a live two-machine lobby (unattended run, no second peer): covered by the unit test on the have-check
  and the existing lobby test (`own_and_downloaded_mods_show_as_themselves`).
- With different versions each player sees their own copy's looks (textures/models may differ slightly); gameplay is
  the same (wire data). Deliberate: the user expects no prompt when both have the mod.
- The "has-it icon" in the report wasn't pinned to a specific widget; the only post-download change the lobby shows
  hinged on the same hash mismatch, which no longer arises for a mod the player has.
- No original-game comparison: online lobbies are ours.

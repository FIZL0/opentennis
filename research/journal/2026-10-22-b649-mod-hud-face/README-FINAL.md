# B649 — mod character's HUD face showed the donor's

## Cause
The match panel picks each player's face once, at its setup: the select's baked close-up of the mod's costume
(`main_menu::face` → the preview cache). A launch waits for the picks' faces only up to a fixed 3 s; a Random draw of a
mod that had never been drawn (its model has to load first, or a store mod's listing picture comes over the network)
could miss that, and the panel then fell back to the donor's INPANE face (`face_CC_OO.tm2`) for the whole match. The
user's cache (`~/.local/share/hst/cache/previews/`) holds only a handful of baked mods, so most mods hit this on a
first pick.

## Fix
- `hst-play/src/play/panel.rs`: a mod player without a face at setup gets a clear 64×64 face, never the donor's;
  `await_faces` retries the select's cache every 0.5 s and, for a store mod's costume 0, asks once for its store
  picture (`modstore::installed_preview`, the select's own drawing) and cuts the face from it
  (`inspect::face_icon`). The picture is written into the same image asset, so the panel, pop-ups and match stats
  (all reading `panel::Art`) show it at once.
- `hst/src/play/main_menu/previews.rs` `picks_wait` + `main_menu.rs` launch: a launch with a mod pick waits up to 8 s
  (was 3 s) for the faces to bake, so a model-only mod normally arrives with its baked face.
- Test: `play::panel::tests::a_mods_face_fills_in_when_its_drawing_lands`.

## Not verified / not 1:1
- Not checked in a live run with a never-baked mod (no visual shot): covered by the unit test of the waiting face; the
  setup branch (mod → clear face) is a plain match on `MatchMod`.
- A local (non-store) mod whose bake doesn't land within the 8 s launch wait shows an empty face for that match (the
  select's baker doesn't run during a match); it is baked the next time the menu is up. Mod character zips carry no
  preview of their own (the standard's `previews/` isn't read for characters), so there's nothing else to cut it from.
- Online launches don't go through the menu's face wait; they get the same clear-then-fill behaviour.
- Menu/HUD presentation: no original to compare (mods are ours).

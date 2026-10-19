# 04 — emotes last their own clip (FINAL)

User: "we should be able to have different timed emotes than donors". The source clips are now about twice the
donor's length (02), so the emote lock, which used the donor's gameplay clip, cut them off halfway.

- `played_out` keeps a human's emote lock going while the mod's own clip (the take being drawn) hasn't played
  out. The donor's length is still the minimum.
- Offline only: online every mod character has `frame_gameplay` set (peers may draw different clips), so it keeps
  the donor's length and the peers stay in step. Stock characters have no own clips: unchanged, and the replays too.
- Test `a_mod_emote_lasts_its_own_clip`: an OOB mod, 2P flicks, held ≈ the drawn take's length (200 vs the donor's 125).

## Not verified / not 1:1
- Not watched in game.
- Online, a mod's long emote is still cut at the donor's length (needs the clip length on the mods' gameplay wire).
- Post-point reactions can still be cut by the next point's timing, which is match timing and stays as it is.
- `a_mod_emote_speaks` failed during this run because the mods in out/mods were being repackaged at the time
  (voice work). Not caused by this change: that test spawns no rig.

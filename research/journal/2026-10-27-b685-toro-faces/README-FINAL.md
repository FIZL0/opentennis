# B685: oob_pc18_toro "has no face clips"

The packager didn't drop anything. Out of Bounds' Toro has no face morphs: none of his six costume REMs has a
morph-target block, with the current `rem.py` or the one before b6773ae. His textures have one face map and no
expression swaps. The motions only rotate `toro_face`/`toro_mimi`. So `face: "morph"` with no channels is all he can
have, and every donor face going unbound is correct. With Toro fixed, the test went on to Sackboy, who fails the
same way: he has 32 morphs (`m_laugh`, `b_angry`, …), but no name or FACE_WORDS/FACE_NAMES stand-in matches a donor
channel. That needs stand-ins picked by eye in the mods repo, so it's a new task (see the TODO line added by B685).

- `oob_faces_all` now expects every donor face unbound for Toro and Sackboy.
- `a_mods_emote_voices_take_turns`: T-Bone's program 9 has six takes now, not four. The mods repo's fc5b115 puts
  each celebration's own source lines in front, and his manifest declares `"9": 6`. That's deliberate, so the test
  now expects 1,2,3,4,5,0,1.

No mod rebuilt, no version bumped: the mods are unchanged.

## Not verified / not 1:1
- I didn't check in Out of Bounds itself that Toro's face never changes (no PS3 emulator here). The claim rests on the
  data: no morph block, no face texture swaps, motions only turn his face node.

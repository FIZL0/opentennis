# 07 — emotes end where they stop moving, and ease out (FINAL)

User: "lots of them have repeat freeze frames at the end. this is fine for celebrations so they dont get cut off but
when the USER emotes it should cut off before the freeze and use the same intro transition to transition smoothly out"

- The freeze frames are the packaging's padding (a clip shorter than its donor holds its last pose). They are exact
  repeats: 307 of the 820 emote clips across the installed mods end in one, by 13 ticks on average at a 1e-3 tolerance
  (293 at 1e-4, 345 at 5e-3).
- A player's emote now holds them until the clip stops moving: per channel, the key after the last one that differs
  from its final value; the clip's length is the latest of these. It is measured from the mod's motions file
  (`mods::emote_ticks`). Offline it's measured when the character loads, online it's sent in the lobby's wire as
  before, so peers still agree. Post-point reactions still play their whole clip.
- Leaving a mod's own clip now eases out over the same 10 frames the ease-in uses, from the pose last drawn into
  whatever comes next (`animate`). The blend is timed on the fixed clock, not the own clip's clock, so the
  disc's clips ease too.
- Tests: hst-base 193, hst-play emote 11 (Anya's emote: held 180 ticks, clip 180 ticks with no padding, donor's 125).

## Not verified / not 1:1
- Not looked at in the running game.
- The ease-out also applies when a reaction is cut off by the next point's setup. That's intended, but nobody has checked it by eye.

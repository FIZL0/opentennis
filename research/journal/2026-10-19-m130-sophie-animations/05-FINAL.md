# 05 — emote length online, ease-in, racket install (FINAL)

User: longer and shorter emotes should work online too, even when the other player lacks the mod (cut short
there if needed); the stance → emote change is choppy; Sophie's racket still went through her face in one emote.

## Emote length, every peer alike
- An emote holds the player for its own clip's length, shorter or longer than the donor's (04 kept the donor's as
  the minimum and stayed on the donor's length online).
- Offline: the length of the take this motion set draws.
- Online: the lobby's mod wire carries each emote take's length in ticks, read from the owner's motions file (the
  last key time × 60, the base clip first, then the takes in K order). Every peer, with the looks or with the
  stand-in, picks the take from the motion-set count (in the snapshot) and holds the player that long. A peer
  drawing the donor's shorter clip shows it held on its end until the lock is over.
- Peer input bounds: at most the four emotes, 1..=64 takes each, 1..=3600 ticks.
- The lock is set where the motion starts (the `motions` system) from `CharacterData::emote_ticks`.

## Ease-in
- Drawing only: a mod's own clip that the game cuts to (no game crossfade) eases in from the last drawn pose over
  10 frames (smoothstep). Gameplay, the snapshot and stock characters are untouched.

## Racket
- The installed Sophie was an older build without racket flags. The new build (hair-touch clip re_gu.3 among the
  flagged ones) was copied in, and the voice agent then reinstalled every mod, version-bumped.

## Tests
- `a_mod_emote_lasts_its_own_clip`: offline, online with the looks and as the stand-in all hold the same, the
  drawn clip's length, well over the donor's.
- `a_mod_emote_speaks` now plays each flick in its own match (the longer emotes outlasted the serve's wait).
- `mods_online` wire check rejects bad emote lengths.
- `doubles_at_100ms_stays_in_sync_sends_and_replays` fails on the snapshot state text: the doubles AI's skipped
  cache `smash_x` (B574, before this work) shows in the state text. Not caused by this change.

## Not verified / not 1:1
- Not watched in game: the ease-in length (10 frames) is a guess, not tuned by eye, emote timing online untested over a
  real connection.
- Shown faces don't ease (the face starts on the new clip's first frame).

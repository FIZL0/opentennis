# B594: one emote voice per character

Each place emotes play already cut its own last line (the lobby/confirm/select plate emotes, B500, and the match's
lines, B473), but each kept its own record: on the character select the stick's up / down (win / loss, B422, the same
two motions as the emotes) played their voice through the previews' record, the left / right emotes through the
plates', so one card could sound both at once.

The record moved into the mixer: `audio::Sound::emote(who, start)` cuts character `who`'s last emote line and keeps
the new one; `stop_emotes` cuts them all (a post-point reaction's line). The plates, the select's win / loss voices and
the match's `play_lines` all go through it. Other characters' lines and every non-emote sound (lock-in voice, umpire,
SFX) are untouched. The select's own rule (a win / loss press dropped while its last win / loss voice sounds) stays.

Checked: `audio::tests::one_emote_voice_per_character` (disc voice bank: the cut line is silent within two frames of
its release, another character's line and a plain sound play on; `stop_emotes` cuts both emote lines only), plus the
existing emote / plate / preview tests.

## Not verified / not 1:1
- Not listened to in the running remaster (unattended run, no audio check); the cut is proved on the mixer only.
- The cross-cut between the select's win / loss voice and a plate emote is ours (the original has no emotes on that
  screen), presentation only: it doesn't touch the sim RNG or match timing.

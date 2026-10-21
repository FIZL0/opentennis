# B619 — emote sound cut at post-point

Before: an emote's voice line was cut only when a post-point *reaction voice* played (B473), and those play only on
the reactions' 5th frame for players the post-point camera has in close view — often nobody. So an emote started
just before the point ended kept talking over the reactions.

Now: `react` (play.rs) queues `emote::Line::Cut(i)` for every player as its reaction starts; `emote::play_lines`
cuts that player's emote line through the new `audio::Sound::stop_emote(who)` (one character, where `stop_emotes`
is all of them). The cut goes through `heard::Played::first` like the other lines, so a rollback re-run of the
reaction tick doesn't cut an emote started later. Sound only: no sim state, RNG or timing touched (the line queue
is drained by the audio system).

Test: `play::emote::match_tests::the_reaction_cuts_the_emote_line` — 2P emotes, the reaction starts, the emote
line stops and no other line plays. Fails with the `Cut` push removed.

## Not verified / not 1:1
- Emotes are ours (not in the original), so there's nothing to compare against; the cut is checked by the test,
  not by ear in a live match.

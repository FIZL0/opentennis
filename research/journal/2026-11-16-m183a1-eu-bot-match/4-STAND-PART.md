# M183a1, part 4: after M190 and M185, both matches part where the game keeps a player standing (PART: blocked on B719)

M190 set up the doubles formation at the start of each point, and M185 ported the stand test's racket read. I ran both
whole-match probes again.

| | US (60 Hz, slot 5) | EU (50 Hz, M177) |
|---|---|---|
| players | all bit for bit through 330 | all bit for bit through 255 (was 249) |
| server's partner after the return | — | six steps bit for bit (250–255), on the recorded tick (M190 fixed the late start) |
| AI generator | as recorded through 330 (M185's test) | as recorded every tick through 260 (was 250), then draws a tick apart at 261/262 |
| where they part | 331: player 1 starts walking (−z); the game keeps it standing | 256: player 2 keeps walking back (−z); the game stops it and it stands from then on |
| ball | parts at 443 | parts at 304 (the next stroke, after the players differ) |

In the EU match the AI draws agree every tick through 260, so the stop isn't a draw difference. Both gaps are the same
kind: the game decides "stand", play decides "move". The US match shows it too, so it's general, not a 50 Hz one. That
is B719, which is open and waits on nothing. I added the EU symptom there as a note so its session checks both probes.
The rule needs the decompile, so it goes through B719's spec, not here.

## Tests (`play::fifty_hz`)
- `the_european_first_serve_set_up_plays_as_recorded`: every player bit for bit through 255 (was 249).
- `the_european_generators_play_as_recorded`: the AI generator as recorded every tick through 260 (was 250), still
  leaving out the set-up's first sample and the serve's contact (140). It prints draws through 265.
- Probe option `HST_M183A1_PLAYER=i,from,to`: player i's ground position, recorded beside played, on both
  whole-match probes. Command: `HST_M183A1_PLAYER=2,244,275 tools/check.sh -p hst-play the_european_bot_match --
  --ignored --nocapture`, then read `context/notes/check.log`.

## Not verified / not 1:1
- **Whole-match proof not done.** Both matches part in point 1 at a stand/move decision (B719): US 331, EU 256.
  `FIFTY_HZ_SIM` is not flipped and there is no release note. Both wait until the EU match plays as recorded.
- **The EU AI generator from 261:** draws land a tick apart (261/262), after player 2 has already parted. I didn't
  chase it.
- **The EU shared and court generators** never match the recording (maybe the recorder), as before.
- **The first 8–9 ticks of the held ball** differ in both builds (the hand pose as the walk starts), as before.
- **The US baseline's `rand()` start is computed**, not recorded (slot 5's intro was skipped), as before.
- **M191's two 50 Hz constants** are still unported, as before.

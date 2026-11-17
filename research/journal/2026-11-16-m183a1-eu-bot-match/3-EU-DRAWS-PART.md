# M183a1, part 3: after M186 and M187, the EU match's generators recorded; it parts at the return's hit message (PART: blocked on M190, M185)

M186 fixed the returner's slide and M187 the return's launch. I ran both whole-match probes again. The return now
flies bit for bit in both builds. Each match now parts at the server's partner (player 2):

| | US (60 Hz, slot 5) | EU (50 Hz, M177) |
|---|---|---|
| returned ball | bit for bit 299–370 | bit for bit 231–303 |
| player 2's first step | on the recorded tick (317), bit for bit | **a tick late** (recorded 250, play 251), same step bit for bit |
| after that step | play pauses a tick, then walks back (−z); the game keeps going | the same |

So two gaps:
- the pause and the walk back, in both builds: M185's stand scratch;
- the EU tick of lateness, which only the EU build has.

## The stand scratch decides the walk (M185)
As a temporary experiment, I set the stand scratch to (0, 0, 1, 0) where the rally routine reads it. That is a positive
depth, which M185 found the game holds there. With it:
- the US partner walks the game's path bit for bit through 322;
- the EU partner walks the recorded steps 250–255 bit for bit, but a tick late.

A depth of −1 makes both walk back as before. So M185's gap is the whole walk-back in both builds. Play still needs the
game's real value, which a write-watch would show. M185 was blocked because slot 5 doesn't load in PCSX2 v2.9.179. The
EU start state (`fixtures/m169/eu-bot-start.p2s`) does load, so I noted that on M185 and unblocked it.

## EU generators recorded
M177 recorded no generators for the EU match. The EE dumps hold the AI generator only at seed 1 (start) and at 3–3
(late).
- New `scripts/eu_rng_rec.py`: loads a slot holding `eu-bot-start.p2s` and records the shared, AI and court generators
  every frame. The addresses are the US ones moved as research/README.md says: AI generator 0x43bfb0, which the start
  dump shows freshly seeded 1.
- I recorded 1300 frames at 0.6× speed. They are in `fixtures/m169/eu-rng.bin` (the shared fixtures folder) and
  `context/m183a1/eu_rng.bin`.
- Command: `tools/pcsx2.sh bash -c 'HST_ISO=fixtures/discs/SCES-54535.iso HST_SPEED=0.6 tools/pcsx2-hst.sh; …
  python3 research/journal/2026-11-16-m183a1-eu-bot-match/scripts/eu_rng_rec.py 8 1300 context/m183a1/eu_rng.bin'`,
  run with the state copied to slot 8.

New test `the_european_generators_play_as_recorded` (in the suite). It asserts that play's AI generator equals the
recording every tick from recorded 2 through 229, the return's contact. Play runs a tick behind the recording, as
everywhere in this harness. Two ticks are left out:
- the set-up's first sample;
- the serve's contact (140), where play's 68 contact draws land a tick apart from the game's. The ticks either side
  match.

This is the first generator check for the EU match. It confirms the match seed (`Seed::after(81_874)`). The shared and
court generators never equal a recorded state. Either the recorder read those pointers too early (at the match object's
first frame), or play's differ. I didn't chase it because nothing has parted on them so far.

## Where the EU match parts: one AI draw at the return's hit message (M190)
The AI draws each tick (recording vs play):
- recorded 228: 5 and 5;
- recorded 230, the return's hit message: **game 68, play 69**;
- recorded 249: 4 and 4;
- recorded 250: **game 1, play 0**;
- recorded 251–255: game 1 a tick, play 4. Play's figure comes with the stand-scratch experiment on.

Play's 69 draws at that message, in order:
- player 0: hit handler 1, timing 16, reaction 1;
- player 1: hit handler 1, timing 15;
- player 2: hit handler 1, timing 16, reaction 1;
- player 3: hit handler 2, timing 15.

The US return draws exactly the same counts, and there play matches the recording. So the extra draw depends on
something this EU point has that the US point doesn't. It could be a 50 Hz rule or a difference between the two matches.

Search: I used a temporary hook (reverted) to make draw j of that message return a dummy without advancing the
generator, as if the game never made it. I ran this for every j, with both dummy extremes.
- j = 0–17 (player 0's draws): player 0 leaves the recording at 249. So player 0's draws are right.
- **j = 18–50: player 2 gets the game's reaction (wait 20, not 21), starts on the recorded tick, and walks bit for bit
  through 255** (with the stand-scratch experiment on). The range covers player 1's timing draws, player 2's hit-handler
  draw (its rally wait restart: the walk-back's return-to-centre roll) and player 2's timing draws.
- j = 51 (player 2's reaction): it parts at 248 or 250.
- j = 52–68 (player 3's draws): it parts at 250 as before.

The game's extra draw at 250, the tick player 2 starts moving, fits this reading: the game makes player 2's
wait-restart draw when the move starts, not at the message. I can't tell the candidates apart from positions until
player 2's walk matches (M185). It needs a per-call AI-draw capture, which the EU start state allows. That is M190,
spec first.

## Other 50 Hz constants I found still at 60 Hz (M191)
- The rally routines' landing helper flies the last path entry on with the US gravity bits. M169's SPEC §7 says the
  European build uses its 50 Hz gravity there.
- The walk-back's re-roll every 60 frames may be one of SPEC §6's "less than 60 → 50" AI helper waits.

I tried both (reverted). Neither changed this point. They are filed as M191, a port from M169's spec, outside this
task's files.

## Not verified / not 1:1
- **Whole-match proof not done.** Both matches part at the server's partner's walk in point 1: M185 in both builds,
  plus M190's extra draw in the EU build. `FIFTY_HZ_SIM` is not flipped and there is no release note.
- **Which EU draw is the extra one**: narrowed to draws 18–50 of the return's hit message, not identified (M190).
- **The EU shared and court generators** never match the new recording. It may be the recorder (pointers read at the
  match object's first frame). Not checked.
- **Two ticks of the EU AI generator**: the set-up's first sample and the serve's contact (140) are a tick apart.
  Both sides of each match.
- **The first 8–9 ticks of the held ball** differ in both builds (the hand pose as the walk starts), as before.
- **The US baseline's `rand()` start is computed**, not recorded (slot 5's intro was skipped), as before.
- **M191's two constants** are unported. They didn't change this point.

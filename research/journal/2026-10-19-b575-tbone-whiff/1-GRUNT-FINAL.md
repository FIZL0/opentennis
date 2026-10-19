# B575 — T-Bone's whiff line too long

**Cause.** `../opentennis-mods tools/rerig/fore_data.py PICKS[17]` gave the whiff (program 4, played at the miss
pose of every first whiff) `sc23`, a per-opponent sentence: "I ain't diggin' this." (0.94 s). It was the only
bad-shot line in his banks, so B562 put it on both 2 (mis-hit) and 4.

**His banks.** Whisper (`out/voice_candidates/.transcripts.json`, keyed by the wav's md5) finds words in every
co/ya/sc/vs/g take of T-Bone, none in `jy0`–`jy4`: those are his wordless shot reactions, 0.30 / 0.52 / 0.58 /
0.94 / 0.99 s. `jy2` is the short bad-shot groan bank (notes/fore.md *Voices*, across all 24 actors), so it is the
whiff now. 0/1/3 keep `jy1 jy2`; 2 (mis-hit) keeps `sc23`, and the sweet-spot / point lines keep talking, as the
task allows.

**Other mods.** Every packaged mod's `voice/4_*.wav` over 0.8 s listed with its transcript: the rest are short
exclamations ("Huh.", "What happened?", "Oh, not good!", "does not compute."), up to 1.15 s, or wordless; only
T-Bone's was a sentence. Left as they are.

**Fix (../opentennis-mods 0e1d847).** PICKS[17][4] = `jy2`; WHY[4] says a groan where the only bad-shot line is a
sentence; `out/mods/fore_pc17_tbone` voice/ + mod.json `voice_counts`/`voice_why` rewritten with
`package.voices()` (as B562), `version` 3 → 4. `voice/4_0.wav` is byte-identical to `jy_tbo2_001.wav`.
notes/fore.md *Voices* says it.

## Not verified / not 1:1
- Not listened to: "a groan" comes from Whisper finding no words and from the jy2 bank's role across the cast, not
  by ear.
- Not re-uploaded to the mod store (unattended run, outward-facing): v4 goes up with the next upload, and installed
  copies refresh then. The mods repo commit isn't pushed either.
- Not checked in a match: game code unchanged (audio.rs plays `voice/4_0.wav` as before).
- Which clip fits a tennis whiff is a judgement; Fore! has no whiff voice.

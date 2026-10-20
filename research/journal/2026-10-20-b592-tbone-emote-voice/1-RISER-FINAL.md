# B592 — T-Bone's alien-sounding emote line

**Which emote.** Emotes play programs 9 (up, big win), 10 (down, big loss), 7 (left, won point) and 8 (right, lost
point), each cycling only over the disc bank's keys (`emote::voice_keys`: 9/10 → 0–1, 7 → 0–2, 8 → 0–4), so a
T-Bone emote is one of 7_0–7_2, 8_0–8_2, 9_0–9_1, 10_0–10_1. All 24 of his packaged wavs map to the right
source cue (md5 against `out/audio/fore`), at the bank's own rate, identical in the user's installed copy (v9) and
the package (v12): no decode, rate or mapping fault.

**Method.** Faster-whisper small (scratch venv; numpy input, PyAV's API had moved) on each clip, the whole and in
parts, plus spectrograms read by eye. Every emote take says words, but 9_1 (`g6`, the second eagle line, "Let the
revolution begin.", 4.4 s) opens on 1.2 s of no speech: a low tone gliding up under broadband hiss (spectral
flatness ~0.7), which Whisper only hallucinates on ("Thanks for watching!", no-speech 0.63), with a faint hum
tail after the words. Over all 144 Fore! `g` takes it has the longest quiet-to-loud intro (1.2 s; next Misaki g5
1.1, Kamala g6 0.9, the rest ≤ 0.8). It is the up-flick's second take, so every other press of 1 / stick up plays it.

**Fix (../opentennis-mods 087f3c5).** `fore_data.py PICKS[17][9]` = `g5 vs01 co.1 vs03` (g6 out, "Yeah, dawg."
`vs03` in to keep four takes); voice/ + `voice_counts`/`voice_why` rebuilt as package.py does
(`more_voices.fill` → `package.voices`), `version` 12 → 13. Up now alternates "Ha ha ha! You know you in trouble
now!" / "Wasn't even close.". notes/fore.md *Voices* says it. Game code unchanged.

**Found on the way.** Takes past the disc's key range never play on an emote (T-Bone's 9_2/9_3, 7_3/7_4): B596.

## Not verified / not 1:1
- Not listened to: "alien" was matched by analysis (no-speech intro, spectrogram), not by ear; the user's report
  doesn't say which flick. If it was another one, the other candidates were all checked to hold clean speech:
  7_2 ("Woo! Ha ha! You heard me!") has a falsetto whoop with harmonics gliding up to ~7 kHz, 7_0 ends on a
  beatboxed "BOOM!" after 0.3 s of digital silence; both are his real celebration lines and were kept.
- Not re-uploaded to the mod store (unattended run, outward-facing) and the mods commit isn't pushed: v13 goes up
  with the next upload, installed copies refresh then.
- Not checked in a match: mod data only, the file names and counts the game reads are as before.

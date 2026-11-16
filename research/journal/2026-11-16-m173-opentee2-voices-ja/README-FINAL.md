# M173 — Open Tee 2 mod voices in Japanese

## What landed (../opentennis-mods 9c57e50, pushed)
- **Rule** (`tools/rerig/voice_langs.py`, `OpenTee2`, in `COPIES`): every Open Tee 2 hats/snd voice bank
  `pc_<kind>_NN<a|b>` exists as `.xb1` (English) and `.xb` (Japanese) for all 21 pcs. Wave *numbers* don't line up
  (pc00 st: the Japanese copy has one wave fewer; pc11 st/vs/ya: one more, shifting the rest), but both copies of a
  bank carry the same SEQD cue slots for the same lines. A take that is an English slot's main wave (the longest
  note, as for Out of Bounds) maps to the Japanese bank's same slot's main wave. The g/gb banks have no region
  table, so they have no slots (the voice picks never use them anyway). Wave names (`Julie_NN` / `Sag_NN`, `pcNN_05A` /
  `pcNN_05A_2`) agree with the slots but were not needed.
- Scan over 126 banks with slots: slot → wave lists are identical in 121; the 5 that differ are the shifts above.
  No Japanese wave has the same bytes as an English one.
- **Mods rebuilt** (live out/mods, versions bumped by the tool): opentee2 pc00–pc20 all got `voice-ja/`. Every
  packaged take has a counterpart except Mika's two takes of her slip line (her last st slot has no Japanese copy):
  they play English.
- `notes/opentee.md` *Voices* explains the rule. No clip lists or transcripts committed.
- No release note: a mod-only update (AGENT.md / release-notes rule). No game code changed (M168's loader is
  generic).

## Verified
- Every take in each opentee2 mod's `voice/` is byte-identical to a hats/snd English wave (so the md5 lookup holds).
- faster-whisper (small, scratch venv) on the 3 longest takes of pc00, 04, 09, 11, 12, 17, 20: English side `en`
  0.84–0.99, Japanese side `ja` 0.90–0.99 (one quiet sigh 0.50; one take detected as `ko`, its transcript is the
  Japanese 「信じられん」 in hangul), and each pair says the same thing ("Nice shot!" ↔ ナイスショット, "That felt
  good!" ↔ いい手応え, "Excellent job!" ↔ 上出来だ).

## Not verified / not 1:1
- Not heard in a running match by ear (unattended run). The loading path is M168's, already tested; presentation
  only, nothing to prove against the original.
- Japanese takes are another actor's performance of the same line. Short shouts/grunts are matched by slot, not by
  meaning; only the longest takes were checked by transcript.
- Mika's slip line (2 takes) has no Japanese slot and stays English.

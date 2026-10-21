# B613 Mod animation speed

Two separate causes, one per mod.

## Bigger Chief (§6 `swing_speed` 2): pace leaked past ground strokes and volleys
M11 meant the swing pace for ground strokes and volleys only (the smash and the serve's swing up to contact stay
contact-timed), but the pace also reached:
- the serve's swing after contact (played at 2) and the serve recovery (halved);
- the recovery after a smash or a dive (halved);
- a smash/dive with no frames left before contact (started at speed 2);
- a smash whiff and a cancelled smash/serve's miss motion (played at 2, whiff tick started 4 in).

Fix: `motion::stroke_pace(branch, rate)` / `motion::anim_pace(anim, rate)` give `rate` only for ground strokes and
volleys (`id::STROKES`), 1 otherwise; play.rs's serve contact, stroke recovery, `whiff`, `miss_contact` and
`whiff_frame` go through them, and `paced_stroke_start`'s frames<1 case is limited to ground/volley. Ground
strokes, volleys, their follow-throughs, recoveries and whiffs stay paced as M11. Rate 1 (every stock character)
is unchanged: both helpers return 1 → identical to before, replay tests untouched. Test: hst-sim `paced_swings`
extended; hst-play `mod_match` passes.

## Lauryn (opentee2_pc16_lauryn): Open Tee 2 clips packaged at 60 Hz, played by OT2 at 30
Measured over two characters' clips: the face track's last key is 1.8–2.0× the body clip's length (as exported at
60 Hz) in every emote, result-screen motion, hole result and reaction. So Open Tee 2 plays these clips at 30 Hz, and
the packaged reactions (re_gu, re_di, the set ones and their takes) ran 2× fast.

Fix (../opentennis-mods 79be029, pushed): `motions.make(..., rate=)` samples the source at `rate` source seconds
per second played; `package.py` `HZ = {"opentee2": 30}` → rate 0.5; manifest `motions_why` says "played at 30 Hz".
Repackaged all 21 Open Tee 2 mods (all share the cause); Lauryn's reactions lengthen (re_gu_set: 108→182
frames). Every OT2 mod's `mod.json` version bumped by 1 (Lauryn 9→10) in out/mods. Her swings are the
donor's (pc12) HST clips, not retargeted from OT2, so untouched (user 2026-10-21: it's the celebration emote).

## The other source games (follow-up, user 2026-10-21)
Same test (face track length vs body clip length, both on the exporter's clock):
- Get a Grip (PSP, same motion format as Open Tee 2): the face track ends exactly where the body clip does (ratio
  1.0 wherever it spans the clip, shorter where it stops early); never 2×. 60 Hz is right.
- Fore! (PS2, HST's own motion format at 80 ticks/frame): face tracks on their own timeline end with their clips
  (ratio 1.0 or a little under, across 144 clips with faces). 60 Hz is right.
- Out of Bounds (PS3): its clock (2400 ticks/s) was already set from the motion itself (4800 made jumps hang 0.2 s);
  its faces share the body's file, so the ratio (1.0) doesn't test it independently.
Only Open Tee 2 needed the 30 Hz rate. Noted in package.py (opentennis-mods, pushed).

## Not verified / not 1:1
- No A/B screenshots or video vs pc07/pc12: playback speed doesn't show in a still; the 30 Hz rate rests on the
  TAT/clip length ratio (consistent ~2× across every clip measured), not on running Open Tee 2 (PSP; no emulator here).
- Out of Bounds' rate rests on the earlier jump-timing calibration, not an independent face/body check.
- The repackaged OT2 mods are not uploaded to the mod store (outward-facing; the user uploads them): installed
  copies refresh once they are.
- Chief's change has no PCSX2 check: a mod's pace has no original to compare against; stock play is unchanged by
  construction (rate 1).

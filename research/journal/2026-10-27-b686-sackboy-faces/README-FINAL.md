# B686 — Sackboy's face stand-ins

His face mesh has 32 targets named by face part, not mood (`m_` mouth, `b_` brows, `e_` lids, `t_` tongue,
`eye_m`), so no FACE_WORDS entry matched and he played no faces. Picked from his contact sheet
(`../opentennis-mods tools/oob2gltf/check.py --face`), in `tools/rerig/rerig.py` FACE_PICKS:

| HST channel | his target(s) | looks like |
|---|---|---|
| blink_eye | eye_m | eye buttons covered: shut |
| joy_eye | e_R_up + e_L_up | lowered lids, a happy squint |
| joy_mouth | m_laugh | wide open grin |
| anger_eye | b_angry | knitted brows |
| anger_mouth | m_R_grrr + m_L_grrr | bared teeth |
| sorrow_eye | b_worried | upturned brows, drooping lids |
| sorrow_mouth | m_R_down + m_L_down | frown |
| doki_eye | b_R_up + b_L_up | raised brows |
| doki_mouth | m_T_ooo + m_B_ooo | open O |

Mods repo (a515ac5, pushed): FACE_PICKS values may now sum targets with `+` (as FACE_NAMES does); `check.py --face`
no longer crashes on untextured materials (his button eyes) and frames the head from the posed mesh (rerigged
models are scaled, so the raw positions missed the crop box). Applied with `face_alias.py out/mods/oob_pc19_sackboy`
(all six costumes got the nine channels), mod.json version 7 → 8. `oob_faces_all` no longer exempts him.

## Not verified / not 1:1
- Picks are by eye from a still contact sheet; not watched animating in a match (no game launch in runner tasks).
- No original to match: the stand-ins are a mod's approximation of HST's mood channels by design.
- The mod was updated in place with face_alias.py rather than a full package.py rebuild (same aliases package.py
  applies; no other part of the mod changed). Not re-uploaded to the store here.

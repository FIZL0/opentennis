# M11 — Bigger Chief

Big Chief (pc07) as a mod: twice as big, swinging twice as fast, running at a quarter of his speed. The mod is a
`mod.json` and nothing else: it ships no model, and the game reads his body, costumes, faces, racket and voice from
the player's disc.

## The mod standard (opentennis-mods `standard/HST-CHARACTER-STANDARD.md` §6)

The standard had no way to say any of this, so §6 gets four optional fields:

- `disc_body: true`: the mod plays its donor's own disc costumes (`disc/c00..c09`), faces, racket and voice. It
  lists no `costumes`, its `face` is `morph`, and it is not a `costume_of`.
- `scale` (0.25–4, needs `disc_body`) scales the whole body:
  - the joint rest translations and the skeleton rest by `s`;
  - the inverse binds by `scale(s) · IB`, so the vertices stay as they are;
  - every motion is re-bound to the grown skeleton, with the pelvis keys × `s` because they're absolute, not
    bone-length-relative;
  - the arm table is read again from the grown strokes;
  - the racket mesh grows by `s`;
  - TParam's size-bound columns (`mods::SIZE_COLUMNS`: height, the height tolerances, the reach
    base/margin/reach, the dive, the body-shot switch, collision, underhand, ideal/smash/serve heights) are
    multiplied by `s` before the overrides.
- `swing_speed` (1–4) speeds up ground strokes and volleys:
  - they start their swing `ceil(8/r)` frames before contact at speed `r` (`motion::paced_stroke_start`,
    `swing_lead`);
  - a swing with less time than that plays at 8/frames as any swing;
  - the post-contact recovery for strokes and the serve is `ceil(n/r)` (`motion::paced`);
  - the follow-through, the serve swing after contact, whiffs (started 4 ticks in at r=2, so the contact pose comes
    when the faster swing's would: `paced_whiff`) and miss motions play at `r`.
  - The per-player pace lives in `Game::swing_pace`, set by `mod_match::apply`.
- `run_speed` (0.1–4): `Stats::paced`, SPE/10 × r. `mod_match::apply` applies it, so the AI, the stamina and the
  locomotion all read the paced stat.
- Online refuses any of these (`Wire::of`): the wire has no fields for them yet.

The select's 3D previews draw a grown body back at its disc size (`CharacterData::size`, `inspect::spawn` / `centre`),
so his card frames his face. Without that, the card showed his belly.

The mod itself is opentennis-mods `tools/rerig/hst_bigger_chief.py` → `out/mods/hst_bigger_chief/mod.json`
(version 1), listed in its README. It's committed there but **not pushed** (the runner rules forbid pushing; push
opentennis-mods `main` to publish).

## Try it

`python3 tools/rerig/hst_bigger_chief.py` in opentennis-mods, then `hst <iso> --play --singles --chars 6,7 --mod
mods/hst-mods/hst_bigger_chief --mod-slot 1`, or pick him on the select: he's the last Hot Shots Tennis card.

## Checks

- `hst-base mods::tests::bigger_disc_body`:
  - the manifest parses;
  - bad values are refused;
  - he's on the roster from a folder holding only `mod.json`;
  - his TParam row is pc07's with the size columns doubled and every other column unchanged;
  - for motions 0x3, 0x10 and 0x20 at four times each, every joint sits at exactly 2 × pc07's position.
- `hst-play mod_match::tests::bigger_chief_plays`: in a bot match against pc07's own numbers:
  - speed is ¼ of pc07's;
  - the swing pace is 2 (1 for the other player);
  - the smash heights are ×2 and the reach is larger;
  - the head's rest height is 2 × pc07's;
  - he hits a ball (frame 1016).
- `hst-sim`: `paced_run_is_a_quarter`, `paced_swings` (rate 1 is exactly the original helpers).
- Shots (`context/m11/run1/`):
  - `match12.png` vs `plain12.png`: he's 2× pc07 at the same spot;
  - `select3c.png`: his card on the roster.
  - The court isn't drawn in this slot's shots, with or without the mod. That's the environment, not this change.

## Not verified / not 1:1

- No PCSX2 comparison: this is an addition with no original to match. With no build fields set, every path runs
  the original's code (rate 1 and scale 1 return the original helpers' values untouched; the full suite's replay
  tests cover mode-off).
- The root paths (the smash and serve jump, the dive) stay pc07's size, and the run cycle plays at pc07's pace, so
  his feet slide at ¼ speed.
- His costume sway (noise deformers) keeps pc07's amplitude.
- The smash, and the serve's swing up to contact, stay timed to contact as any character's (they're contact-driven,
  so they can't be "2× faster" without moving contact). A whiff's re-press lock and recovery after the miss pose
  keep their frame counts.
- His shadow and the footstep dust use the game's generic sizes.
- The select's 3D preview and card show him at normal size, by design: it's a framing fit.
- Online play is refused for him. The wire format would need the build fields first; there's no follow-up task
  unless the user wants one.
- Cached select pictures key on the mod folder's contents, so an install that baked him before this change (only
  this slot did) would keep the old belly crop until `mod.json` changes.

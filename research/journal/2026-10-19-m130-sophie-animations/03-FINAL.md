# 03 — racket hidden where it would go through the body (FINAL)

User: in some of the new reactions the racket sticks right through the character; hide it there, keep it elsewhere.

- Game: an own clip or take with animation `extras.racket: false` hides the `Racket` joint (its visibility) while it
  is the drawn clip (`CharacterData::racketless`, keyed by motion and take like `own_faces`; `character::racket_shown`
  after `animate` in the match). Test: `takes_load_and_take_turns` checks the flags land on the right takes.
- Mods repo (dd22b32): `motions.py` flags a clip when ≥2 of 9 points along the racket are inside body capsules
  (spine to head top r 0.13 m, hips r 0.12, thighs r 0.08) for over 12 % of its frames. Calibrated on stick-figure
  plots of two OOB mods (front and side views of the worst frames): a fist-pump with the racket through the face,
  a hand-on-chest pose with it through the torso, a hand-on-hip pose with the shaft through the hips are flagged;
  brief pass-throughs stay.

## Not verified / not 1:1
- Thresholds judged from stick figures, not the skinned mesh: wide costumes (dresses, big hair) may still clip.
- Hidden for the whole clip, not only the frames it would clip; the racket pops back when the next motion starts.
- Character-select previews and the ceremony don't use the flag.
- Not yet looked at in the running game.

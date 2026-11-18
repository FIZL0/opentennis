# M185 3: the spec's open questions answered

Spec phase only (no Rust). SPEC.md now has an empty *Open questions*; the port session works from it.

## What was found (research notes; the SPEC is clean)

- The pose hand-off copies the matrix of the first of the 17 model nodes the player looks up at load. The name table
  (read from the EE RAM fixture) starts "Racket", "Bip01Head", "Bip01Neck", …: it is the **Racket** node's world
  matrix. The node world build is local × parent world, rooted at the object's placement (set from the player's
  world matrix right before the hand-off).
- The player update's order: AI/move → motion step → post-step pose adjust (rally state 0 or 1: the contact arm
  turn on hand/forearm/upper arms; with the dive solve's flag also a racket-local turn about y by angle × weight)
  → placement → hand-off. The dive's racket angle is set by the dive IK set-up (the N6 dive journal's unported IK
  step). Follow-up **B716** ports it.
- Play already has: `node_world` (the held-ball chain), `player_matrix` (mirror in row 0), the posed nodes
  (`bodyhit::Posed`, but kept only for `Bones::SEEN` chains) and `ArmIk::apply`. The port must keep the Racket
  chain and have player s−1's pose of this tick before player s's AI.
- Rain: `scripts/eu_rain_rec.py` forces the court's weather byte to raining after loading the EU start state
  (slot 8), then logs the dispatcher's dead stack (capture `context/m185/rain/rain1.bin`, 600 frames). Slot 0:
  walker value 214×, (0,0,0,1) 3×: the old value just persists. No negative z for slot 0 anywhere.
- Skip flag: the main loop's scene-start flag, set on a scene request and cleared by the match's set-up; never set
  during a point.
- Unnamed tick (EU 38329, slot 2): previous player's contact frame; return address in the shared library area;
  neighbouring racket z positive both sides, so the sign agrees.
- Smallest racket |z| over 2577 unit-direction calls in eu_stack.bin: 0.0079.

## Not verified / not 1:1

- No port yet: not shown that feeding the Racket node's row-0 z makes the US probe run past point 1 (the port
  session does that).
- Dive racket turn: what sets its angle and the dive's arm turns are only outlined; B716 specs and ports them. Until
  then the player updated after a diving player can read a different sign on the dive's frames.
- Rain: checked by forcing the weather byte mid-match, not on a match that started on a rain court (no such save
  state); a match that starts raining reads whatever was last written at that depth, observed values all z ≥ 0.
- The unnamed tick's writer routine is not named (only that it ran during the hit).
- US slot 5 still not captured live; all live numbers are from the European bot match.

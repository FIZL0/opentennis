# B510 — post-point camera inside a court mod's helicopter

User 2026-10-21: on Dubno (court mod `gag_c20_city`, layout 3) a post-point camera sits inside the helicopter.

## What it is
- The helicopter is a gimmick of the mod's `gimmick.glb` (a placed piece under `game_space`), parked just past the
  right doubles sideline: cabin box at rest x 7.1–10.0, up to 3.7 m, z −6.9…9.0; rotor disc 4.2–4.6 m up over the
  court's side.
- None of the existing passes reaches it: B240's `buried` and B303's `in_front` only run on the disc court's props
  (and the disc court is hidden under a mod); M8f5a3's `clear_view` only on `__clear` pieces; M129's shader cut keeps
  everything inside the players' room box (x < 9.485), which takes in most of the cabin, and only cuts within 1 m of
  the eye; B506 only leaves out creatures.
- Logged (temporary `HST_CAMLOG` hook in `net_hide`, removed) the drawn camera on Dubno, then tested each frame
  against the helicopter's mesh boxes offline:
  - 10 min all-CPU doubles (`--pads -,-,-,-`): 480 frames from one fixed cut-away eye at x 10.43, 3.0 m up, z 1.84,
    looking back across the court through the cabin (0.47 m past its side).
  - 4 min doubles with P2 keyed (`--pads -,k,-,-`, KeyJ + KeyD walks right after each point, as the user
    suggested): 1213 frames from the same eye.
  - Singles (CPU): a camera passing through the rotor disc's height (3 frames); a telephoto replay whose zoomed B303
    reach takes in the rotor.
  - Earlier runs with a keyed P1 never served (the user: keyed runs go on P2, in doubles).
- `context/b510/heli_off.png`: that eye, the cabin fills the frame. `heli_on3.png`: with the fix, the court, the
  umpire, the WIND CAUTION flag and a crow (other gimmicks) stay; only the helicopter goes.

## Fix
- `court_mod::NearPiece` (hst-base): for `gimmick.glb` each placed piece (the children where the single-child chain
  from `game_space` ends; `__clear` ones left to `clear_view`) keeps its meshes' rest boxes that reach over 0.5 m up
  (its ground shadows left out) and what to hide (its node, its skinned meshes).
- `net_hide::hide_near_pieces` (hst-play): hides a piece whole while one of its boxes comes within 0.5 m of the drawn
  camera's line out to 3 m (B303's `in_front` at r = 1). Not the zoom-scaled reach: at a telephoto's r ≈ 8 it took a
  caution flag 20 m away, in frame behind the player (`context/b510/tele_ab.png`); at r = 1 it stays
  (`tele_on2.png`). Off while `cull::Off` (intro flyover). `drawn_view` is shared with `hide_around_eye`.
- Tests: `net_hide::tests::gimmick_in_front` (the logged cut-away hides the cabin box; a far box on a telephoto's
  line stays at r = 1 though B303's reach takes it), `court_mod::tests::dubno_helicopter_is_a_piece` (`HST_MODS`:
  the helicopter is one piece, its boxes take in the logged eye's cabin, shadows out, no `__clear` piece).

## Not verified / not 1:1
- **No comparison with the original.** Dubno is a Get a Grip (PSP) court drawn by a mod; HST has no such set piece
  or camera view, and Get a Grip's own camera isn't available here. The fix is B303's fallback, as the task allows.
- **Whole helicopter hidden**, not cut: the cut-away from that eye shows no helicopter at all, rather than the part
  outside the lens. Same as B303 does for disc props.
- **Rest boxes.** Animated pieces are tested by their boxes at rest (the rotor spins inside its rest disc box; the
  flags wave a little past theirs).
- **A telephoto standing a few metres back from a piece** (beyond 3 m) that fills its frame isn't caught; none was
  seen in the logs.
- Only `gimmick.glb` pieces: `court.glb` blockers stay with M129's shader cut and `__clear`.

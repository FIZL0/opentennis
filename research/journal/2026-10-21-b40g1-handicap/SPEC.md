# B40g1 spec: Set Handicap (run-speed handicap and leg weights)

This is the clean-room spec. It describes in words what the original does. The port session works from it, the
recordings and the oracle.

## Summary

The original's confirm screen has a Set Handicap page. On it each player in the match gets one of three settings:
None, Handicap x1 or Handicap x2. The setting does two things:
1. **Run speed.** The player runs at 100 %, 80 % or 70 % of its normal speed, for humans and CPUs alike.
2. **Looks.** The player wears one weight (x1) or two weights (x2) on the lower legs, for the whole match.

Nothing else in the game reads the handicap: not the shots, the stamina, the AI's choices or the scoring.

## The setting

- One value per player slot (players 1–4 in the order the match numbers them, the same order as the character and
  costume picks). 0 = None, 1 = Handicap x1, 2 = Handicap x2.
- Menu text (message0.dat in MENU00B.XB0):
  - option names: 298 "None.", 299 "Handicap x1.", 300 "Handicap x2.";
  - help for the confirm row: 327 (handicaps and offbeat rules together) and 330 ("Set handicaps. Characters wearing
    weights cannot run quickly.");
  - per-player help: singles 335–336 ("Set Player 1's / 2's handicap."), then 337 (Apply); doubles 338–341 (Players
    1–4), then 342 (Apply).

  So the page lists one row per player in the match (2 in singles, 4 in doubles, CPUs included), then Apply. The
  front end is ours to design (PLAN.md). The original only serves as a reference for the labels and the help.

## Run speed

- When a match is set up, each player gets a **size %** (the value the sim calls `size`):
  - 100 for None;
  - 80 for x1;
  - 70 for x2;
  - 100 for any other value.
- The size % stays fixed for the whole match.
- The game's run-speed rule already takes this % as its last factor: speed × (size / 100), after the tiredness
  factor. `hst_sim::player::run_speed`'s `size` argument is that factor. The original applies it in two places:
  - **a human's run step:** the velocity of a run on the pad, as `run_speed` computes it;
  - **every CPU estimate and move that uses a run speed:** the reach estimate (`ai::Runner::frames_to`, oracle
    `ai_run_frames`), the rally movement, and the doubles routines (`rally::Body::size`).

  All of them read the same per-player %. None of them adds any other handicap rule.
- In the port today, every caller passes 100 or sets `size: 100`: the two in `hst-sim/src/player.rs`'s mover, the
  run in `play.rs`, the two `Body`/runner set-ups in `doubles_ai.rs`. With the handicap, each of them uses that
  player's own size %. With every handicap at None, the numbers are the same as today, so the replay tests stay
  exact.

## The weights (looks only)

### Models
- Four models on the disc, in `AZUMA/C_EFF/EFFCT.XB0` (`data/azuma/chara_eff/weight/`):
  - `weight`: the default;
  - `weight_pc05`: for JJ;
  - `weight_pc07`: for Big Chief;
  - `weight_pc09_c`: for Kent in costumes 5–9.

  Each is a plain MDL (with its MTL/MTI). There are no motion or UV tracks.
- Which model a player gets:
  - JJ → `weight_pc05`;
  - Kent in costume 5 or higher (0-based, costumes 5–9) → `weight_pc09_c`;
  - Big Chief → `weight_pc07`;
  - everyone else, Kent in costumes 0–4 included → `weight`.

  (Characters by the menu's order: Ashley 0, Cody 1, Jun 2, Kaito 3, Momoko 4, JJ 5, Carol 6, Big Chief 7,
  Miranda 8, Kent 9, Lola 10, Will 11, Gloria 12, Suzuki 13.)
- **How many:** a player has two weight slots:
  - slot 0 is filled when the handicap is 1 or more;
  - slot 1 is filled when the handicap is 2.

  So x1 gives one weight (slot 0 only) and x2 gives two. Both slots of a player use the same model.
- The weights are created when the match is set up. The game rebuilds a player's weights only if that player's
  character or costume changes. A match has no such change, so for the port they are fixed for the match.

### Where they sit
- Slot 0 belongs to the **right calf** bone (`Bip01RCalf`), slot 1 to the **left calf** (`Bip01LCalf`). So an x1
  player wears a single weight, on the right calf.
- Every frame, a weight is placed only if:
  - the player has a handicap;
  - the player's body was drawn that frame: it is visible and passed the game's view check (the body's own
    1 m–1000 m test).

  The player raises a "draw my weights" flag in its own draw. The weights read it and clear it again. So the
  weights show whenever the body shows: match camera, cut-aways, replays. They never show without the body.
- **Placement row.** The row is the character number for costumes 0–4, and character + 14 for costumes 5–9. Each
  row holds:
  - a normal offset (three numbers along the bone's own x, y, z axes);
  - a mirrored offset (three numbers);
  - a uniform scale for the weight model.
- **Unmirrored body** (right-handed characters, drawn as modelled):
  - The weight's matrix is the slot's calf bone's world matrix, taken whole (same axes).
  - Its position is the bone's position minus the normal offset, applied along the bone's axes: position −
    ox·(bone x axis) − oy·(bone y axis) − oz·(bone z axis).
- **Mirrored body** (the left-handed characters, whose body model is drawn mirrored):
  - The weight uses the **other** calf's bone: slot 0 takes the left calf, slot 1 the right calf.
  - Its matrix is that bone's world matrix with the x axis negated. The y axis, z axis and position stay as they
    are.
  - Its position is that bone's position minus the mirrored offset along the new axes. The x term is along the
    negated x axis.
- The scale is set on the weight model. It sizes the mesh about its own origin and does not move the placement.
  A scale of 1 means none.
- Each weight is drawn with the player's own lighting/draw context, the same one the body uses. So it is lit and
  shaded like the player wearing it.
- Rows (normal offset x, y, z · mirrored offset x, y, z · scale). Rows 14–27 are the same characters in costumes 5–9:

  | row | character | normal | mirrored | scale |
  |---|---|---|---|---|
  | 0, 1, 2, 4, 6, 8 | Ashley, Cody, Jun, Momoko, Carol, Miranda | 0.15, 0, 0 | −0.3, 0, 0 | 1 |
  | 3, 10, 12 | Kaito, Lola, Gloria | 0.2, 0, 0 | −0.35, 0, 0 | 1 |
  | 5 | JJ | 0, 0, 0 | −0.15, 0, 0 | 1 |
  | 7 | Big Chief | 0.14, 0, 0 | −0.25, 0, 0 | 1.3 |
  | 9 | Kent | 0.2, 0, 0 | −0.35, 0, 0 | 1.15 |
  | 11 | Will | 0.25, 0, −0.015 | −0.4, 0, −0.015 | 1 |
  | 13 | Suzuki | 0.02, 0, 0 | −0.18, 0, 0 | 1 |
  | 14–27 | same as rows 0–13 except row 23 | | | |
  | 23 | Kent, costumes 5–9 | 0.275, 0, 0 | −0.385, 0, 0 | 1 |

  The values are in game units (metres), along the calf bone's local axes as the game's skeleton has them.
- The original's mirrored case was written for its own way of drawing left-handers. The port draws lefties its own
  way (`character.rs`, handedness from character data). So in the port, a lefty must end up with the weight(s) on
  the same visible leg(s) as in the original. Check this by eye against a Carol or Will screenshot rather than
  copying the axis flip blindly. This is rendering, so "looks right" is the bar (PLAN.md).
- Mod characters: the original has no rule for them. A mod character takes its donor's row and model (the same data
  path as a stock character, PLAN.md *Additions*). The weight goes on its own calf bones by name. A rig without
  calf bones simply shows no weight, while the run-speed handicap still applies.

## Order within a frame

1. Players move. The run speed uses the size %.
2. The player draw runs. A visible, handicapped player raises its weight flag.
3. The weights' draw places and draws the weights of each flagged player (slot 0, then slot 1), then clears the
   flag.

The weights never touch the sim, the RNG or match timing.

## Verification for the port

- **Run speed:** an oracle check of the run speed with size 80 and 70, through `ai_run_frames` (it already takes the
  player's size), over at least 10k inputs. Plus one app-level test: a handicapped player's run step equals
  `run_speed(…, 80/70)` and an unhandicapped one stays as before. The replay tests stay exact with every handicap at
  None.
- **Looks:** compare `tools/screenshot.sh` with `--shot` at the same moment: one weight (x1) and two weights (x2) on
  a right-handed character, on a lefty (Carol or Will), on JJ, Big Chief and Kent in costume 5 or higher.
  `research/journal/2026-10-08-b40-menus/scripts/b40g_confirm.py` walks save slot 2 to the original's confirm
  screen.
- Online/netplay: the handicap changes gameplay, so it has to be part of what the peers agree on at match start,
  with the other match settings.

## Open questions

none

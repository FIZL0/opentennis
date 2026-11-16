# M185: slot 5, point 1: where play's AI generator leaves the game after the return

## What happens

- Play and the game agree through the return's contact (recorded tick 298). The first AI draw that differs comes from
  the **server's partner** (computer, net player, partner also a computer) at recorded tick 318.
- **The game** (`fixtures/ai_net_s05.bin`, the post-scratch-fix recording of slot 5's match): at recorded 317 and 318 the
  partner still has no stand target. It runs the stand search (4 draws a tick) and walks with the stick
  (0.7844, 0, −0.6203). At 319 it presses a dive: 9 draws (219 → 228), and its rally substate goes to the swing. At 320
  it holds its position (no draws). At 321 it moves again.
- **Play**: at play tick 319 (recorded 318) the routine's "the path is past its second bounce, on the other half from
  the scratch target" test passes. So play re-picks with the partner's shot cue, sets the follow flag and returns.
  That is 6 draws, a tick before the game's dive. Every draw after that is shifted.

## Why

- The NET/BASE routines start the stand search from a scratch target the game never initialises. Whatever the last
  routine at that stack depth left there is what the test reads. `Rally::stash` models it as the routine's own
  target from its last call.
- Play's value at recorded 318: (3.755, 0.898, −8.327, 1), its own stand from the tick before. Its depth sign is
  negative, so for side 1 the test passes.
- The game's value at recorded 316..322 for this player is a **unit direction** with w = 0 that turns smoothly from
  tick to tick: (−0.958, −0.106, 0.266) → (−0.904, −0.154, 0.398) → (−0.832, −0.206, 0.514) → (−0.738, −0.263, 0.621)
  → … Its depth is positive, so the test fails and the partner keeps searching.
- Over the whole fixture, the scratch value seen at entry is never the routine's own previous target:
  - Teams 1, 2 and 3: a unit direction on about 97% of calls. The rest are zero or something else.
  - Team 0: one constant position-like quad with w = 1, (8.8869, 0, 12.9595, 1).
  So something earlier in the frame writes it, per player: most likely the previous player's update, because the
  player update calls the AI update almost first thing.
- No match among the directions between the player, partner, opponents, ball and the other recorded vectors
  (normalised differences; checked at 316..318).
- `ai_rally_s05_all.bin` shows zero on every call, but it was recorded before the recorder's stack-scratch fix
  (T2 journal), so its zeros are the recorder's own and not the game's. The earlier "zero every call" idea from
  that file is wrong.

## What unblocks it

A write-watch capture on slot 5 to name the routine that writes that scratch slot (the 16 bytes 0x20 below the AI
update's stack pointer) before the partner's AI update, frame by frame through point 1. With the writer known, the
spec can say what direction it is (some player's facing, aim or bone direction) and play can supply it.

**Slot 5 can't be loaded now:** PCSX2 v2.9.179, upgraded 2026-11-15, rejects save states made with v2.9.52
("no longer compatible"). Every user slot was saved with the old version. The yay cache has only the new AppImage.
The human needs to re-create slot 5 (the bot-only doubles match at the same point) in v2.9.179, or provide a
v2.9.52 binary.

## Not verified / not 1:1

- Which routine writes the scratch direction: not found. It needs the live capture above, which is blocked by the
  save-state version.
- Points after point 1, and the rest of point 1 after recorded 318: not checked. Every draw after 318 is shifted.
- The returner's slide before contact (recorded 291–298): play moves about 3× too far per tick (0.04585 vs 0.015355).
  The generators still agree through 298, but this probably explains why the return launch is about 1e-4 off and
  why player 0 is about 6e-4 off. Not chased because the generator divergence comes first. It needs its own task.
- No Rust shipped: temporary diagnostics in play.rs, doubles_ai.rs and fifty_hz.rs were reverted.

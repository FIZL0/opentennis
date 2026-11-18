# M185 port: the stand test reads the previous player's racket

Port phase, written from SPEC.md with the decompile locked.

## What landed

- `crates/hst-play/src/play/doubles_ai/scratch.rs` (new). Before each doubles computer player's rally update, play
  fills the rally's leftover quad (`Rally::stash`) with z = what the spec says the player finds:
  - slot 0: +1;
  - slot s ≥ 1: hand(s−1) × z of the world x axis of player s−1's `Racket` node;
  - on the tick player s−1 tossed: the toss velocity's z;
  - on the tick player s−1's hit started the trail: +1.
- How the racket is built: the chain from `Racket` up to the root, from the motion pose. The contact arm turn
  (`Player::arm`, slerped by its weight) is applied to the four arm joints, then the player's placement
  (`player_matrix`, whose row 0 carries the hand).
- `after_pose` (FixedUpdate, after `bodyhit::pose`) saves each player's chain and the tossed flag and shot count at
  the end of the tick. `bodyhit::Bones::SEEN` now also looks up `Racket`, and `Posed::nodes(i)` exposes the posed
  nodes.
- **Timing (the `AHEAD = 2` look-ahead).** Play's posed nodes are the game's pose from two motion frames earlier
  than the one the stand test reads. Without a correction, the recorded z equals play's racket z two ticks later,
  to 4 digits. So `after_pose` runs the motion player on two more frames, using the same clock, crossfade and held
  fade logic as `character::tick` and `bodyhit::pose`, restricted to the chain's nodes. With that, the chain saved
  at the end of play tick k (and read at play tick k+1's control) equals the recorded scratch z at recorded tick k
  on most ticks, e.g. 238: −0.9724 for both.

## Results (slot 5's US bot match, 60 Hz; `the_us_bot_match_plays_as_recorded`, ignored test)

- **Before:** play's AI generator left the recording before tick 320 and never found it again.
- **After:**
  - Through 330, the AI generator equals the recording every tick except 218 and 329.
    - 218 is the serve contact, where play draws two early. This was already the case before M185.
    - At 329 play is one tick late.
  - Through 426, play's AI generator stays on the recording's draws. From 387 some draws land a few ticks apart.
  - At 427 it leaves the recording.
  - The test now asserts all of this.
- **Point 1:** a player now parts from the recording at 331 (was 318 per the first-serve test's comment), and the
  ball at 443.
  - At 331 player 1, the server's partner, starts walking while the game keeps it standing.
  - The z it reads that tick (0.618) has the same sign as the recorded one (0.631), so the stand test's branch is
    the same and the cause lies elsewhere. Follow-up: **B719**.
  - From 332, slot 2's z flips sign (rec +0.54, play −0.28) only because player 1's racket moved with it.
- **Sign agreement of the z the stand test used against `fixtures/ai_net_s05.bin`,** per call through 427: 1049
  calls, 63 differ.
  - Through 330 only 4 differ: 214 slot 3, 300 slot 3, 309 slot 2, 314 slot 2. The game's value there is ≈0 or
    a tiny ±0.02, or its pose is offset (below).
  - The rest come after the 331 divergence.

## Method

- `scripts/racket_vs_rec.py fixtures/ai_net_s05.bin <racket.txt> <maxtick> [ai=slot,...]` compares the recorded
  stash per AI object with play's per-tick racket z values. Those are lines `racket K [z0, z1, z2, z3]` from a
  temporary print of each player's racket after every tick in `played_ticks_rng`. The print was removed before
  commit; to rerun, add it back.
- The AI pointer to slot map for slot 5: 0xc6e1e0 = 0, 0xca8ca0 = 1, 0x1074470 = 2, 0x1275fb0 = 3.

## Not verified / not 1:1

- **The dive racket turn is not ported** (B716). On ticks a player dives, its racket reads the undived pose.
- **The toss and trail exceptions are implemented as the spec says but not hit in this recording's checked
  window,** so they are unverified against the game.
- **The `AHEAD = 2` look-ahead predicts the motion.** If a motion switch happens inside the two predicted frames,
  the prediction uses the old clip, so those ticks read the wrong pose. This is a stand-in for play posing in step
  with the game's AI. Signs agreed on every tick checked before 331 except the four above.
- **Small value gaps, same sign:**
  - Around 225–236 and 248–264 play reads about −0.83/−0.79 where the game has −0.78. This could be the arm
    turn's weight timing or idle timing.
  - Player 2's idle loop runs about 4 ticks out of phase with the game, which shifts slot 3's values. That is a
    motion-timing gap outside M185, and the signs still match.
- **Only checked offline against the existing slot 5 fixtures.** No new live PCSX2 capture was made.
- **Only the US 60 Hz match was checked.** The European 50 Hz bot test still passes, but its scratch values were
  not compared.

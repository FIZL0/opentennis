# B700a spec: a computer server's flat serve and the press frame

Applies to the singles computer server (measured live). The doubles serve state runs the same aim step in the
decompile, but it wasn't recorded live. Human serves are unchanged.

## Summary

Whether a computer serve comes out flat **does not depend on when the AI presses**. The swing kind it draws, the
stick it aims with, and so the topspin → flat turn, come out the same for any timing error. Only the swing's
timing offset changes. No new rule has to be ported. The port phase adds a test that pins this down and checks
two details of the draw order (below).

## Which stick the serve contact reads

1. During the serve swing, the computer server holds the stick **centred on every frame but one**.
2. On the **aim frame** it writes its serve aim (the swing-kind-dependent zone pick and jitter the port already has
   in `ai::serve_aim_singles` / `serve_aim`) as the stick. The aim frame is the swing's last frame before contact,
   the frame on which the swing's countdown to contact reads 1, and the AI's update runs before the swing resolves.
   So the contact is resolved **on that same frame**, with that stick. The stick goes through the pad's stick
   bytes (×127, rounded half away from zero, +128). That only rounds the stick; the direction test below gives the
   same answer.
3. The contact turns the swing's ✕ (topspin) into flat when that stick points within 60° of straight at the net
   (`shot::stick_kind`, as now). Live: the pressed-button record changes from ✕ to "flat" on the contact frame,
   only when the aim-frame stick points that way (e.g. (−0.38, −0.78) and (+0.41, −0.58) toward the net turned
   flat; (−0.84, 0), (+0.70, 0) and (−0.01, 0) stayed topspin).
4. The AI's rally routine never supplies the serve's stick. The server hands over to its rally routine only once
   the serve stroke is over (the stroke-over flag clears). That comes more than 30 frames after the contact (live:
   the serve state was still running 31 frames after the ball left the racket).

## Why the press frame changes nothing

- The swing kind (topspin / slice / the weak-toss swing) is drawn at the press. In the original nothing else
  draws from the AI's generator between 8 and 4 frames before contact, so the draw gives the **same value
  whether the press comes 8, 7, 6, 5 or 4 frames out**. Live: re-timing the same serve from err −4 to +4 never
  changed the swing kind (15 of 15 serves).
- The aim's draws (side and depth zone, angle jitter, length) happen on the aim frame, at the same ball frame
  whatever the press, and read nothing about the timing.
- So for a given generator state at the toss, the timing error moves only the swing's timing offset
  (`−max(err, 0)`, B700). The kind, the aim, the stick and flat-or-not stay the same.

## Evidence

- **Intervention** (`research/b700a_err_poke.py`): CPU Will (row 11) against a mashing human Ashley, save slot 8
  of PCSX2 copy 1. At each CPU toss the state was saved, then the same serve was replayed with only the serve
  timing error forced (−4, 0, 1, 2, 3, 4, or −4, 0, 4). 15 serves in two matches, 8 or 5 replays each. In every
  serve all replays gave the same swing kind, contact stick, button after contact, aim point and ball kind.
  Only the press countdown (8, 8, 7, 6, 5, 4) and the offset (0, 0, −1, −2, −3, −4) moved. Two of the serves were
  flat. One was a late press (err 4, offset −4) and stayed flat when re-timed to offset 0. The other was clean
  and stayed flat at offset −4. A plain replay with the error untouched gave the same result twice (the replay is
  deterministic).
- **Per-frame trace** (`research/b700a_serve_stick.py`, 10 serves): the stick word is centred on every swing
  frame except the contact frame. The swing countdown and the AI countdown run together from the press. The
  button turns ✕ → flat on the contact frame exactly when the stick points within 60° of the net.
- **Observational share** (`research/b700a_flat_share.py` over six `b700_serve_press.py` runs, 52 topspin
  swings, 4 of them repeated across runs): flat 3/31 at offset 0, 9/21 at offsets −1..−4 (Fisher p ≈ 0.02).
  The intervention shows the press frame doesn't cause this. It is chance, or a side effect of where the timing
  error and the aim fall in the AI generator's sequence. An exact port of the draw order gets that for free,
  with no rule. Overall: 12 flat of 52 topspin swings (23 %). The will-curve notes' "10 of 15 late serves flat"
  came from a mislabelled column (B700 README): their "offset" was the serve's **depth error**, which a
  forward-pointing stick (the flat one) shows. That picked flat serves out for the "late" rows.

## What the port should check and test

1. **Test: re-timing a CPU serve keeps its kind.** In an all-CPU singles match, at a CPU serve's toss snapshot the
   game. Replay that serve with the server's serve timing error forced to −4, 0, 2 and 4, and assert the swing
   kind drawn at the press, the contact stick, the final shot kind (incl. flat) and the serve target before its
   timing error are the same in all four. Assert that the timing offset is `−max(err, 0)` (needs B700's press
   rule). Use a seed or a serve where the result is flat, and one where it isn't.
2. **Draw order the test may expose** (fix it if it does):
   - The swing-kind draw must give the same value at any press frame from 8 to 4 frames before contact. So no
     other draw on the AI generator may fall between those frames in the server's or receiver's routine. If the
     port's receiver (or anything else) draws on the AI generator every frame there, the kind will change with the
     press frame and the port is wrong in that draw order.
   - The aim's draws come on the contact frame itself, before the contact. Check that the port's aim tick (today
     "the frame before the contact", `swing.left <= 2`) is the same frame of the original's timeline: the
     same draws in the same order relative to the receiver's routine on that frame. If the existing serve-aim
     recordings already prove the order, note that in the journal and change nothing.
3. No change to `shot::stick_kind` or to which stick the contact reads. The port already uses the aim stick it kept
   for the serve (`serving.bot_aim`), which matches.

## Open questions

none

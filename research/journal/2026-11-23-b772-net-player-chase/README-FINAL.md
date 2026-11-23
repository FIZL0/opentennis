# B772: the EU net player's chase at point 3's tick 1585 (answered; no port of its own)

Behaviour is in [SPEC.md](SPEC.md). This is the session summary.

## Result

The game uses the existing no-reachable-contact fallback. On tick 1585 (game rally tick 411) the after test reads a
leftover direction z of +0.2663 (non-negative), so there is no partner or middle repick, and the net player chases the
after-bounce entry 39 (4 draws). Play reads −0.0021, the same racket-direction sequence two ticks behind, and takes the
middle repick.

The gap is in the leftover racket direction model's European timing (look-ahead), not in the searches. The rule is
common to both builds. The value gap is European-only so far: a look-ahead of 4 breaks the US match, and the European
lead changes between swings (0 or 4 frames).

## How it was found

1. Offline: a trial forcing play's leftover sign non-negative at 1585 gave draws 4, 6, 4, 10, the run to entry 39,
   players to 1588 and the ball to 1885.
2. Live (PCSX2, European disc, save slot 8):
   - `scripts/eu_code_dump.py` dumped the overlay, to locate the US routines in the European build by instruction
     pattern.
   - `scripts/eu_stack_tick_rec.py` hooks a routine entry or a call site and logs the dead stack (default), or a quad
     an argument register points at (`SRC=a1`, `SRC_END`), plus the game tick (`AIREG=s5` at a call site whose delay
     slot sets a0). It waits 5 s after the state load because a 1 s wait let the load overwrite the hook.
   - `scripts/eu_stack_tick_show.py` decodes the dead-stack records.
   - The captures (in `context/b772/` run6, run8, run11, run13) gave: slot 2 leftover per tick; the after test's input
     at ticks 411–414; which routine runs and when the fallback is reached (not on 409 or 410); per-frame draw counts
     3+7, 4, 10 at 411–414.
3. Trials with look-ahead 0–5 in play: 3–5 give 16 of 144 EU phases (was 7), 0–2 give 7; 4 breaks the US match.

## Problems hit

- PCSX2 hung on a 1700-frame capture, and once after hooking the landing-short helper. Its second instruction is a
  branch, so it can't take an entry hook; I hooked call sites instead.
- `tools/pcsx2-hst.sh stop` didn't end a hung copy.
- The European state needs PCSX2 v2.9.179 (`HST_PCSX2_BIN=pcsx2-qt`), not the slot's v2.9.52.

## Not verified / not 1:1

- What sets the European racket pose lead per swing (B773).
- The dive at tick 1588, where play's net player doesn't move but the game's does. Not looked at.
- The game's leftover was captured for this one rally phase only.
- Tick alignment between game rally ticks and recorded ticks comes from the draw pattern (offset 1174) and is checked
  only in point 3.

## Follow-ups

- B-task for the European racket-direction lead (scratch.rs): B773
- B-task for the 1588 dive movement: B774 (after B773)

## Closing

B772 needs no port of its own: the fallback is already in the port. It closes as answered rather than going to a port
phase. The fix is B773's. M183a1 now waits on B771, B773 and B774.

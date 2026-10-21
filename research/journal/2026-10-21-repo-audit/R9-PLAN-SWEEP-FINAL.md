# R9 — Plan sweep (from the repo audit, section 9)

Only the plan changed: `plan/TODO.md`, `plan/DONE.md`, `plan/HUMAN.md`, plan files. `tools/ctx.py` needed no change.

## Done

- **Already handled by the user (2026-10-21), skipped:** P16, P35, P17p1, B107 (folded into B72), Z3, M18. None of
  them is in TODO.md any more. So the P17p1/P16 rewording no longer applies.
- **P11k6 / P11l6:** their TODO lines were already open, but `plan/P11k6.md` and `plan/P11l6.md` (which `ctx.py <ID>`
  shows before the TODO line) still said `[~]` with the old "waits on the ball path port" block. Reopened them there
  with "P11n1 is done". Dedupe: P11l6 no longer lists P11k6's shared gaps as its own work. It says they are ported once,
  in P11k6. Each TODO line now matches its plan file (the k6 TODO line had lost the "net repick on a hit" item).
- **P11:** it stays `[~]` as the user's umbrella. The stale first block reason ("subtasks k4–k6/l4–l6/m1 blocked") is
  gone from the TODO line and from `plan/P11.md`. Only the umbrella reason is left (P11k6, P11l6, P116). I didn't fully
  unblock it, because an open parent would show up as a task of its own.
- **Stray B395 panic fragment:** the two loose lines after B510 (`…bevy_render-0.19.1/…/mod.rs:1241:40:` /
  `index out of bounds…"`) are deleted.
- **M136:** already ticked in DONE.md, next to B601 and M141. Nothing to do.
- **M7a:** the user dropped it in 154f314f (2026-10-14, the M5/M6 reshuffle). The only remaining mention is the
  audit itself, so no live reference was left to fix.
- **P23:** it was a real task. 2ab0df48 added Settings → Graphics and the `HST_FPS` log. 31573209 did the Deck
  profiling. The user dropped the rest as a dead end in c2b2c16c (2026-10-16), and that took the line out of both lists.
  `crates/hst/src/main.rs` (log_fps), `hst-play/.../popups.rs` and `tools/bench.sh` still name it. Rather than edit
  those comments, I added a `[x] **P23**` record to DONE.md next to P23b. It says what landed and when it was dropped,
  so `ctx.py P23` resolves.
- **Orphan plan files:** these 19 files were deleted. They are the audit's 18 plus `P16.md`, which became an orphan
  when the user dropped P16. List: N1, N1d, N1d3, N3, N3d1–4, P0b, P0b4, P0c, P10, P11k, P11l, P16, P17, P23, P7, P8.
  Nothing outside them pointed at them; they only pointed at each other. Their text stays in git history. A rerun of
  the orphan check finds none.
- **HUMAN.md:**
  - *Teammates and the ball (N5a):* removed. P11n2's spec (journal 2026-10-19-p11n2-partner) answers it: both
    teammates can lock the same ball, and the first strike, hit or net touch cancels the other lock.
  - *Bot serve cases (P11i):* removed. P11i is ticked. Case 1 (the return of serve) was reached by P11l4's vpad-driven
    serves. Cases 2–3 (a bot's second serve, the serve-and-net-dash aim) have no recording yet. Agents can drive these
    themselves now, so they went onto P116 as a note instead of waiting on the human.
  - *4-player re-record (P27):* removed. The user said it is no longer needed (c2b2c16c).
  - *P13 umpire long match:* removed. P13 is ticked, and umpire calls are presentation (feel-right rule), so no
    recording is required.

## Not verified / not 1:1

none (plan bookkeeping only, no game behaviour touched). The 132 plan files of done tasks with stale checkboxes inside
(audit §9) were left alone because R9 didn't ask for them.

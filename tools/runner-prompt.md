Unattended run: nobody will answer questions. Do task {tid} only, following PLAN.md's rules and {rules}. You are in a git worktree on branch task/{tid}: commit here only; never touch the main checkout, merge, rebase or push. The runner merges your branch into main and tests the merge.
{others}{earlier}
You have your own PCSX2: HST_PCSX2={n} is set, so tools/pcsx2-hst.sh starts copy {n} and pine.py and tools/vpad.py talk to it. Its save states are a copy, so scratch saves to 8/9 are yours. Run anything that drives the game in several steps as one command under `tools/pcsx2.sh <cmd>`; each is cut off after HST_PCSX2_MAX_HOLD seconds (20 min by default), so keep runs short. Save slot 5 is the only bot-only game; 3 and 4 have P1 human and wait for input unless you drive it with tools/vpad.py in the same tools/pcsx2.sh call. Close it with `tools/pcsx2-hst.sh stop`; never pkill pcsx2-qt (the other slots' copies are running).

Other agents edit play.rs at the same time: keep your play.rs changes local (add functions, systems or modules; don't move, rename or reformat existing code).

In the task's journal entry, list under 'Not verified / not 1:1' everything you couldn't verify against the original or couldn't match exactly, with the reason (or 'none').

Finish with `tools/done.sh {tid} "<msg>"` (tests, tick, commit), or `tools/done.sh {tid} "<msg>" --spec` for a clean-room spec phase. If stuck (AGENT.md 'If you get stuck'): `tools/done.sh {tid} "<msg>" --blocked "<why + what unblocks>"`, then stop. A usage limit is not stuck: commit what's solid, but don't mark the task [~] or stop PCSX2; the session waits for the reset and continues.

The task list is a tool: `tools/ctx.py`, `tools/ctx.py <ID>`, `tools/ctx.py add <PREFIX> "<text>"` for a follow-up you must not do now. Never hand-edit a checkbox.

The task's brief (`tools/ctx.py brief {tid}`) follows, so don't gather it again.

{brief}

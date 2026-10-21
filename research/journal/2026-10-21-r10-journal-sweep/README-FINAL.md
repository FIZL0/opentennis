# R10 — journal sweep (from the 2026-10-21 repo audit)

## Done
- The 16 journals whose newest file was `-READY` for a ticked task (p13-umpire, m2-netcode, m2j-lan, p27-4player,
  b244, b118, b256, b272, b273, b274, b305, b310, m20-chat, p30, p33, b515): newest file renamed `-READY` → `-FINAL`
  (`git mv`), and the links to it in the dir's README and `plan/HUMAN.md` updated. Older `-READY` iterations below a
  newer file are history and stay as they are.
- The 4 uppercase-ID dirs lowercased (b54-player-teeth, b427-mode-pill-glyph, b490-lobby-no-sides,
  n3c7-lob-whistle-tail); references in `plan/DONE.md` updated.
- `research/journal/2026-10-08-b63.md` → `2026-10-08-b63-menu-same-process/README.md`.
- Decision on the ~113 suffix-less dirs and ~280 README-less dirs: no mass rename. AGENT.md (*Research*) now says a
  journal is a `README-<STATE>.md` or numbered `N-<TOPIC>-<STATE>.md` files (README optional then), old dirs with no
  state suffix anywhere count as `-FINAL`, and the plan's checkbox wins if they disagree (e.g. P6 is `[~]` with a bare
  README.md). Renaming 400 historical dirs would only churn git history and the links into them.
- `tools/done.sh`: on a tick (not `--blocked`/`--spec`) it renames the task's own journal's newest state file from
  `-READY`/`-PART` to `-FINAL` (dirs `YYYY-MM-DD-<id>[-…]`, case-insensitive) and, in the main checkout, adds those
  dirs to the commit. Tested in a scratch repo (prefix-ID dir `x55` not matched, `10-PART` > `2-…` by version sort).

## Left as they are
- `-READY` with no task ID in the slug, so not checkable against the plan: 2026-10-05-ball-physics, 2026-10-09-p23-graphics
  (P23 is a section, not a task), 2026-10-15-landing-page. `-PART`: 2026-10-06-characters, 2026-10-14-android-apk,
  2026-10-15-p35-high-fps (no P35 in the plan).
- b554/b555/b559 end `-READY` and are `[~]` blocked: correct.

## Not verified / not 1:1
none (tooling/docs only).

# T11 — slot-only recordings into the main checkout's `context/`

T5 found 13 recordings that hst-sim's tests need. They existed only in some runner slot's own `context/`.
`hst_data::fixture` reads them from any worktree, but deleting the slot would have lost them.

## Done

- `tools/context_sync.py`: copies this worktree's `context/` files into the main checkout's `context/` when main
  lacks them. `--all` does the same for every worktree. The copies are reflinks (FICLONE), with a byte copy as
  fallback, and they keep mtimes. An existing file is never overwritten. When two worktrees have different files
  under the same new path, the first one (in `git worktree list` order) keeps the path and the other is kept as
  `<name>.from-<worktree>`. `--since EPOCH` also replaces main's copy of a file this worktree rewrote after EPOCH.
- **Backlog.** I ran `tools/context_sync.py --all` once: 186,118 new files (192 GB) went into
  `/home/ryha/repos/opentennis/context` in 19 s. Free space on /home went from 889 to 888 GB (reflinks).
  232 same-name different files were kept as `*.from-<worktree>`: b102 123, b101 99, p17v 6 (`sim_tile*.npy` from
  s4/s5), mods 2, p3d1 1, plus one build log.
  All the files the task names are now in main: `p17v/cull.txt`, `p17v/rcv_pass.txt`, `b174/orig/palais.txt`,
  `p3f/`, `p3f3/run2.bin`, `p3d4/call_s05.bin`, `b134/cloth.bin`, `p3f4/setup_ram.bin`, `p3d1/`, `p3e2/`, `p14d/`,
  `p17l/`. Main's `context/` is now a superset of every worktree's paths, so any file the T5 fallback found
  elsewhere is in main.
- **Recorders write there.** The first `Pine()` in a process registers an exit hook. On a clean exit it runs
  `context_sync.py --since <start> -q`. The new files and any rewritten ones (a re-recording under an old name) reach
  main as soon as the tool ends. A crashed run (`CAPTURE FAILED`) or one cut off by the hold cap is skipped, so its
  incomplete output doesn't replace main's copy. In the main checkout the sync does nothing (0.4 s scan otherwise).
- `tools/done.sh` runs the sync (new files only) at the start of every wrap-up in a worktree, even before failing
  tests stop it. This covers what non-PINE scripts wrote (`research/tools/fixture_*.py`, screenshots).
- Docs: research/README.md (*Worktrees* paragraph under *Rebuild `context/`*) and the `fixture.rs` module doc.

## Checked

- Scratch repo with two worktrees: missing files copied; an existing file kept; a conflicting new file kept as
  `.from-b`; a directory symlink carried as a link; `--since` replaced a rewritten file; `-q` silent with nothing to do.
- The pine.py hook, called directly: copied a new probe file into main; with the failure flag set it copied nothing.
  I removed the probe afterwards.

## Not changed / why

- I didn't turn the slots' real `context/` directories into the runner's symlink (`runner.toml` `links`). With a
  shared `context/`, every slot would write the same `context/notes/check.log` and since.rev. Agents are also using
  those directories right now.
- `--all` keeps main's copy where main and a slot differ under the same pre-existing path. Mtimes can't tell a
  re-recording from a stale copy (s5's copies of main's fixtures carry newer mtimes with identical content). Such
  pairs are left alone and not counted.
- The individual recorders weren't edited: the pine.py exit hook covers all of them (all but `trace_agg.py`,
  which writes no recording, use pine).

## Not verified / not 1:1

- No gameplay change, nothing to compare against the original.
- I didn't run a recorder against PCSX2 to watch the exit hook fire end to end. The hook's function was tested
  directly, and `Pine()` only registers it.

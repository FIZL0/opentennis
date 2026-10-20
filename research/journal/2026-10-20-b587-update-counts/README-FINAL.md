# B587 — Mod store Updates view: kind tab counts

The user saw "Characters 77" in the Updates view when no mod needed an update. The grid's list was already right:
Show's Updates pill filters it down to installed mods whose store zip is different. The kind tabs' counts were not.
On Browse they counted every store entry of each kind and ignored the Show pill.

Fix: on Browse, the kind tab counts now go through the same Show filter as the list, so Updates and Saved mods
count only what they list. Installed, My uploads and Upload work as before. The headings above the tiles come from
the list, so they already showed only updates.

Test: `grid_sorts_filters_and_moves` now checks that the Characters tab counts 1 under Updates (the one mod with an
update) and 5 under Browse.

## Not verified / not 1:1
- Not checked against the original: the mod store is ours and has no original to match. Checked only by the unit
  test, not in a live store run (unattended run, no live store login).

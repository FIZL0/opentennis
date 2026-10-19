# B579b: spring hair overwritten by the mode's animate

Sophie's (OOB) pigtails stayed rigid on the select card, in the viewer and in matches, even after B579's solver change.

**Cause:** `spring::swing` is ordered `.after(character::animate)`, but the match and the select cards run their
`animate` inside the mode's own schedule (`mode.rs`: `|w| run(w, Update)` in the main Update). So that ordering does
nothing, and the two were left unordered. When 3b127eab added `mips::plugin` to Update, Bevy's topological order
changed and the mode runner moved after `swing`. From then on `animate`'s `set_if_neq` reset every hair joint every
frame. v1.2.1 predates that commit, so its hair still worked.

**Fix:** the mode's Update runner is ordered `.before(crate::spring::swing)`. Test `mode::tests::the_mode_poses_before_the_hair_swings`
fails without it.

**Evidence** (context/b579b/): bisect log and per-commit card crops in `bisect/`; springs on/off pairs in `run2/`
(the card's pigtails hang with springs on and stick out with them off). In a match, springs on vs off differ by
453 px with the fix and 65 px on main.

B579's solver changes (SAG ×0.3, every link keeps lagged inertia) were diagnosed while this bug was active. They are
left as they are for now: whether to go back to the B410b tuning is the user's call.

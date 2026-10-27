# B687: a_mods_emote_voices_take_turns vs the live mods folder

The test hard-coded T-Bone's take counts per emote program, so every repackage of the mod with more or fewer voice
takes broke it (B681's run: program 9 went from four takes to six). Main had already bumped the numbers once
(44d1e4a8); this time the test reads the counts from the mod's own `voice/` folder (`<program>_<take>.wav`) and
expects the emote to go take 1, 2, …, n−1, 0, 1 for each of programs 9, 7 and 8. It still needs at least two takes
of each (asserted with a clear message) and still skips without `mods/hst-mods`.

Checked: passes on the current mod (6/5/3 takes); a deliberately wrong cycle (mod n+1) makes it fail, so it isn't
skipping.

Also: done.sh's clippy step failed on main's B675 code (`hst-sim/src/look.rs` float negation outside `F`,
`tests/look.rs` `chunks_exact` with a constant size, `hst-base` `character.rs` a collapsible `if` in the head-turn draw). Fixed all three in place (negation is exact, so the head-look result is
unchanged; its bit-exact recording test still passes).

## Not verified / not 1:1
none (test-only change; no gameplay).

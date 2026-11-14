# B704 — split_view tests panic on DirectionalLightShadowMap; patches.py fails

Both problems were already fixed on main by aeb2d9c5 (B702) before this task ran:

- `twin_view::shared_cascades` takes the shadow map as `Option<Res<DirectionalLightShadowMap>>` and returns when it's
  missing (headless test apps have no shadows to share).
- `third_party/bevy_pbr/src/lib.rs`'s comment reads `opentennis patch: …`, which `tools/patches.py`'s marker
  accepts; the README row naming `src/lib.rs` (`ReusesShadowMaps`) was already there.

Checked: `tools/patches.py` prints nothing; `tools/check.sh -p hst-play own_camera` → 7 tests pass (including
each_team_has_its_own_camera and each_human_has_their_own_camera). No code change.

## Not verified / not 1:1
none (test/tooling only)

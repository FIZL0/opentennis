# B583 — no culling during the match intro

The intro's flyover drives the same `Orbit` camera as the match, so every view-dependent hide ran on it:

- **B303's "prop right in front of the lens" pass** (`net_hide::hide_around_eye`, `in_front`): its sample line
  reaches 3·r ahead, r = tan 20° / tan(half fov) up to 15, so the flyover's zoomed and low shots hid trees, fences
  and props as it swept past. Logged on court 1 (`HST_INTRO=1 --stage 1 --court 1 --play`): ~1,000 hides
  (`near=true`, r 1→2) over the ~5 s flyover. This is the vanishing.
- the original's buried-object/net test (same system, and `hide`, which reads the *match* camera, not the
  flyover's) and court 5's end-stand crowd test (`npcs::cull_crowd`).
- Bevy frustum culling of the `Cullable` meshes (characters, effects) on their joint-following bounds.

Fix: `hst_base::cull::Off`, a resource. While it exists, `cull::apply` unculls every `Cullable` mesh (and new spawns:
`ON` now holds the setting and `Off` together) and the three hide passes above leave everything shown. `intro::uncull`
inserts it while the flyover shows and removes it at the cut to the match camera. The banner hold after the cut
already uses the match camera, so culling there works as in the match.

Check: re-ran court 1's intro with a temporary log: `Off` came and went once, and nothing was hidden while it held.
After it went, only the match camera's existing buried-object hides showed up, same as before. Flyover frames
(`context/b583/run3/`) are complete. Test: `cull::tests::off_unculls_till_it_goes`.

## Not verified / not 1:1

- No frame-by-frame A/B of Bevy's frustum culling on players during the flyover: the logs and frames don't show
  where its bounds would have popped. It's off for the flyover anyway, as the task asked.
- Static court meshes keep Bevy's frustum culling: their bounds are fixed boxes, so culling them can't pop anything
  on screen.
- The original runs its net/buried-object test on the intro's view too; we no longer do (user's call: nothing hidden
  during the intro). Presentation only.

# B583 — no culling during the match intro

The intro's flyover drives the same `Orbit` camera as the match, so every view-dependent hide ran on it:

- **B303's "prop right in front of the lens" pass** (`net_hide::hide_around_eye`, `in_front`): its sample line
  reaches 3·r ahead, r = tan 20° / tan(half fov) up to 15, so the flyover's zoomed and low shots hid trees, fences
  and props as it swept past. Logged on court 1 (`HST_INTRO=1 --stage 1 --court 1 --play`): ~1,000 hides
  (`near=true`, r 1→2) over the ~5 s flyover. This is the vanishing.
- the original's buried-object/net test (same system, and `hide`, which reads the *match* camera, not the
  flyover's) and court 5's end-stand crowd test (`npcs::cull_crowd`).
- Bevy frustum culling of the `Cullable` meshes (characters, effects) on their joint-following bounds.

- **Mod courts' camera-object clearing** (the user's report, 2026-10-20): `court_mod::clear_view` hides a
  `__clear` set piece standing between the eye and the court (M8f5a3), and gs.wgsl's `GS_CLEAR` (M129) discards a
  big blocker right in front of the eye. Both ran on the flyover's sweeps.

Fix: `hst_base::cull::Off`, a resource. While it exists, `cull::apply` unculls every `Cullable` mesh (and new spawns:
`ON` now holds the setting and `Off` together) and the hide passes above leave everything shown. For mod courts, `clear_view` shows every `__clear` piece, and
`court_mod::clear_off` turns `GsKey::clear` off on every material that has it, catching ones that load late, then
back on when `Off` goes. `intro::uncull`
inserts it while the flyover shows and removes it at the cut to the match camera. The banner hold after the cut
already uses the match camera, so culling there works as in the match.

Mod court check (Fighill Tennis Park, `--court-mod …/gag_c05_street`): a temporary log showed all 140 clearing
materials going off at the flyover's start and back on at the cut. Flyover frames match `HST_CLEAR=0` by eye
(`context/b583/run4/`; the elevated railway in front of the lens is drawn).

Check: re-ran court 1's intro with a temporary log: `Off` came and went once, and nothing was hidden while it held.
After it went, only the match camera's existing buried-object hides showed up, same as before. Flyover frames
(`context/b583/run3/`) are complete. Test: `cull::tests::off_unculls_till_it_goes`.

## Not verified / not 1:1

- Mod courts: no pixel A/B of the flyover with and without clearing. Real-time shots land on slightly different
  flyover frames each run, so they can't be diffed. Checked by the material log and by eye instead.
- Toggling `clear` changes the court's pipeline key, so the first intro on a machine may compile the other variant
  at the flyover's start or end (once; pipelines are recorded and pre-warmed after that). Not measured.

- No frame-by-frame A/B of Bevy's frustum culling on players during the flyover: the logs and frames don't show
  where its bounds would have popped. It's off for the flyover anyway, as the task asked.
- Static court meshes keep Bevy's frustum culling: their bounds are fixed boxes, so culling them can't pop anything
  on screen.
- The original runs its net/buried-object test on the intro's view too; we no longer do (user's call: nothing hidden
  during the intro). Presentation only.

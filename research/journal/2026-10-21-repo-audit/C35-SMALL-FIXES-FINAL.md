# C35 — small code-quality fixes from the audit

## Done

- **Menu `Settings` has named fields.** The 12-slot tuple and its `[bool; 9]` + index constants (`UPSCALED_DOWNLOADS`,
  `KEY_HINTS`, …, `SETTING_NAMES`, `SETTING_DEFAULTS`) are gone: one field per setting (`music`, `chat`, `graphics`,
  `aspect`, `downloads`, `updates_always`, …), defaults in `Default`, and `Settings::switches()` pairs each on/off
  field with its settings.txt name (parse and text both walk it, so the file's order and words are unchanged). Tests
  build settings as `Settings { music: true, ..base }`, so adding or removing a setting no longer touches them.
- **`hex`**: roster.rs re-exports frame.rs's `const fn hex` (same arithmetic) so its importers stay as they were.
- **`now()` ×3** (net.rs, modstore.rs, net/rejoin.rs) → `hst_base::unix_secs()` (web_time, so the browser's clock on
  the web; net.rs's old one went through `n0_future`'s SystemTime, rejoin's through `js_sys::Date`). The rejoin record
  always stored whole seconds, so it now compares whole seconds too.
- **tokio**: `hst_base::runtime()`, one lazily-built multi-thread runtime. The mod store's `spawn`, the self-updater's
  check and install threads and the tests in modstore.rs / update.rs / main_menu.rs use it instead of building one each
  time. The store and updater still run each job on its own OS thread (`runtime().block_on`): installs unzip with
  blocking file I/O, and on a worker that could hold up other tasks.
- **hst-sim**: `cross` is in `camera.rs` once; cut-aways' 4-wide `cross` pads that with w = 0 (same chopped products and
  differences). Left alone, because they round differently: player.rs `dot`/`cross` (madd/msub accumulator), player.rs
  `rot_y` (axis-angle with madd), world.rs `rot_y`/`rot_x` (VU0 game sine, built in a zeroed register) vs cutaway.rs's
  (musl sincosf), cutaway.rs `mul` (FPU chop) vs world.rs `mat_mul` (VU0 transform) vs pose.rs `mul` (host floats).
- **`i32_at`/`f32_at`**: one copy each in `hst_data` (lib.rs); mdl.rs and hst-sim's replay.rs use it.

## Kept on purpose

- `Net::start` keeps its own runtime (2 workers). A Net lasts one online session, and dropping that runtime when the
  session ends cancels the lobby's detached `n0_future::task::spawn` tasks (listings, pack readers, relays). On the
  shared runtime those tasks would keep running after the menu drops `Net`. A comment there says so.
- The net tests' own `Runtime::new()`s are unchanged, for the same reason: each test's tasks end with its runtime.

## Checked

`tools/check.sh` on hst `main_menu` (254), hst `update`, hst-base `modstore`, `rejoin`, `net::`, hst-sim
`--test cutaway/camera/replay`, hst-data. `cargo clippy -p hst --tests` clean; `cargo check -p hst-base --target
wasm32-unknown-unknown` clean.

## Not verified / not 1:1

- No gameplay change, so nothing was checked against the original. The `cross` merge does the same F ops in the same
  order, and the cutaway/camera tests pass.
- Mod-store downloads and the updater on the shared runtime: only their unit tests ran. Nobody tried a real store
  download or update in the running game.
- The netplay match tests (`HST_NETPLAY=1`) weren't run. Net's runtime and its match code are unchanged; only its
  `now()` moved.

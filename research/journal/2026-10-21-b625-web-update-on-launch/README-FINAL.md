# B625 — Web app: check for updates on launch

**Why it missed it.** A home-screen web app opens the build the service worker cached. The check behind B317/B470
only offered a new build once sw.js had downloaded all of it in the background (`version.txt?next`), or after
3 minutes behind the server (`?server`). iOS stops the service worker during that ~70 MB download, and phone
sessions are short, so the player never saw it.

**What changed**
- `web/index.html`: on launch, before any of the game loads, the page asks right away. If sw.js has a newer build
  ready, the page reloads into it (the navigation promotes it). If only the server has one, the page drops the
  `hst*` caches and reloads from the network (the same thing the existing Refresh does with `hstFresh`). Guards:
  this happens at most once per build per tab (`sessionStorage hst-picked`), so a CDN still serving the old copy
  can't make it loop. It's skipped when `navigator.onLine` is false, and each read gives up after 3 s, in which
  case the cached build plays.
- Resume (`visibilitychange` → visible) calls `check(true)`: a newer build on the server is offered at once (the
  New version card / site panel), without the 3-minute wait. The game doesn't reload by itself mid-session, since
  that could interrupt a match or unsaved menu changes.
- `update::updated()` (`crates/hst/src/update.rs`): the browser keeps the last build it ran in localStorage
  (`hst-build`). On the first main menu of a different build, `takeover::updated` shows the existing toast pill
  for 4 s: "Updated to v1.3.0 - 28656f1", worded like the version corner. Nothing shows on a first visit.
  `HST_UPDATED=1` fakes it on desktop for `--shot`s.

**Checked**
- `cargo check -p hst` (native and wasm32) passes. `node --check` passes on the page's module script.
- A node harness ran the launch block with fetch, caches and storage mocked. Same build → boots. Ready newer →
  reload. Server newer → caches dropped, reload. Already picked → boots. Offline → boots. Server read hangs →
  boots after 3 s.
- `HST_UPDATED=1 HST_MENU=main tools/shot.sh` shows the toast at the top of the main menu
  (`context/b625/updated.png`).

## Not verified / not 1:1
- Not tried on a real iPhone home-screen app, or end to end with two deployed builds in a browser (that needs a
  device and a second deploy). The logic was only checked with mocks.
- Whether iOS fires `visibilitychange` when a home-screen app comes back from the background depends on iOS. If it
  does, the offer is now immediate. If iOS killed the page instead, the next launch picks the build up.
- Desktop and mobile native builds don't show the "Updated to" toast after a self-update: the task covers the web
  app, and those builds' updates already restart with their own progress text.
- Menus/front-end aren't 1:1 with the original (PLAN), so there's nothing to match there.

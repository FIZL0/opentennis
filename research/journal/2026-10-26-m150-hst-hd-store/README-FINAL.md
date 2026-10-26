# M150 — Hot Shots Tennis HD pack on the store

**Answer: it is on the store, at v1.2, and it lists.** Nothing had to be uploaded.

## What I checked (2026-10-26)
- HF PR 151 (`refs/pr/151`, author hotshotsgaming) is **open**, not merged. That's normal: an open PR is how an
  upload goes live on the store (unverified). Its head is 946d576d. Its commits include "Update mod Hot Shots Tennis
  HD (hst_hd)" at 03:26 and 03:32 UTC on 2026-10-26, a few minutes after the rework's texture files were written
  (2026-10-25 23:20 EDT = 03:20 UTC).
- `mods/hst_hd/meta.json` at refs/pr/151: version 1.2, 2091 textures, zip 1,531,927,191 B, sha256 ee347e7c….
  `index.json` on main (rebuilt 04:40 UTC, "103 mods, 1 open PRs") holds the same entry, at the PR's current head
  commit, so the listing uses the index's copy as it is.
- The store's `mod.zip` is the installed rework: its central directory (fetched with range requests) has 4310
  entries. They match `~/.local/share/hst/mods/hst_hd` file for file: same names, CRC32s and sizes, `mod.json`
  included. The only local file not in the zip is `store.json`, the install stamp, which is never uploaded.
- Desktop listing: a scratch `#[ignore]` test (removed afterwards, not committed) ran the real `lists()` → `merge`
  → retired filter against the live store. It returned 103 mods, and the one `Kind::Textures` entry is
  `hotshotsgaming/hst_hd` "Hot Shots Tennis HD", refs/pr/151, v1.2, sha ee347e7c…. That is the Texture packs tab's
  only store entry. The user's own app wrote the same entry into `~/.local/share/hst/mods-store.json` at 06:07
  today.
- Browser: the three URLs the listing reads (index.json through its LFS redirect, the open-PR list, refs with PRs)
  all answer a cross-origin request with `access-control-allow-origin`. The browser runs the same listing code.
- B670 (the five screens) hasn't landed yet. If it rebuilds keys into the pack, it bumps `mod.json` and re-uploads.
  I left a note on B670 saying so.

## Why it might have looked missing
- A Browse opened before 03:32 UTC, or a browser tab whose hour-long localStorage cache (`mods-store.json`) predates
  the upload, shows the earlier listing.
- The installed copy's `store.json` is still the 2026-10-17 install stamp (sha 6d44197e…, the earlier pack). Uploading
  from the Upload page doesn't rewrite it. Browse still tags the store entry "Installed", not "Update": the versions
  are equal (1.2 vs 1.2). So the stamp does no harm.

## Gate fix
- Clippy (new lints) failed on two lines of `crates/hst-sim/tests/cutaway.rs` from B665, on main too: `&shots` → `shots`, `chunks_exact(STRIDE)` → `as_chunks::<STRIDE>()`. No behaviour change.

## Not verified / not 1:1
- I didn't look at the Texture packs tab on screen, on desktop or in a browser. `HST_MENU=browsemods` shows made-up
  entries, not the live store. The desktop check ran the listing code itself against the live store. For the browser,
  I only checked CORS on its endpoints; I didn't build or run the wasm build in a browser.

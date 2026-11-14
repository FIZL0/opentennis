# B710: the self-updater brings vulkan/ along

B708 put Mesa's lavapipe (`vulkan/`) into the Linux tarballs, but the in-game updater only fetches the bare binary,
so a game updated in place (or into the user data folder's `update/<tag>/`) had no software driver beside it and
Rendering → Software fell back to Hardware.

## What landed

- `tools/release.sh` publishes `opentennis-linux-x86_64-vulkan.tar.gz` and `opentennis-linux-aarch64-vulkan.tar.gz`
  (each tarball's `vulkan/` alone). They match the `opentennis-*` glob, so they're in the signed `SHA256SUMS` and
  uploaded with the rest.
- `update.rs`: `Found` carries the `<binary>-vulkan.tar.gz` URL (Linux assets only). After an install succeeds,
  `vulkan()` makes sure the new game's folder has `vulkan/lvp_icd.json`: left alone if there; copied from the running
  game's folder when that has one (a tarball install whose update went to `update/<tag>/`, no download); else the
  tarball is downloaded, checked against the minisign-signed sums like the binary, unpacked with `tar` into
  `.vulkan.new/` and renamed into place (no half-made `vulkan/` on failure). A failure only logs: the binary update
  still goes ahead.
- The *old* binary runs the update, so installs updated by a pre-B710 updater still arrive bare. `mend()`: a release
  build on Linux with no `vulkan/` beside it fetches its own release's JSON (`releases/tags/<VERSION>`) once at start,
  in the update-check thread (so not with Check for Updates off / `--no-update-check`), and adds `vulkan/` the same
  way; it's used from the next start. A release from before B710 has no tarball: nothing happens.
- Settings → Graphics → Rendering: when Software couldn't start and `render_mode::bundled_missing()` (Linux, no
  `vulkan/` beside the game), the row says the software renderer isn't in the game's folder, that the game fetches it
  by itself while checking for updates, or to download the game again and unpack it over this one.
- Test `update::tests::updates_bring_the_software_renderer` (local HTTP server, a second throwaway minisign key, a
  208-byte stand-in tarball): tarball path, wrong key leaves nothing, copy from the running folder, an existing one
  kept, release without one, `mend` on Linux vs Windows assets.

## Not verified / not 1:1

- Not 1:1: updater/release tooling, nothing from the original applies.
- Not run against a real GitHub release: no release has been made with the new assets yet (only the local-server
  test). `release.sh` was syntax-checked (`bash -n`), not run.
- Unpacking uses the system `tar` (gzip): present on every desktop distro and SteamOS; a system without it logs the
  failure and keeps Hardware, with the Settings text pointing at re-downloading.
- Only a missing `vulkan/` is fetched: a later release with a newer lavapipe won't replace an existing one (Mesa is
  pinned in tools/lavapipe.sh; a bump would need a version check here).
- A system-wide install the player can't write (and that has no `vulkan/`) can't be mended: logged only.

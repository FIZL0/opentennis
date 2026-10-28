# M155a1 — beta builds carry the PortMaster zip

`pages.yml`'s `linux-aarch64` job now installs `zip` after `tools/linux-aarch64.sh setup` (readelf is already there:
binutils comes with gcc), runs `bash tools/portmaster.sh` after the build, and uploads `opentennis.zip` with the
tarball in the `linux-aarch64` artifact. The `beta` job's `gh release create` attaches `opentennis.zip` beside
`opentennis-linux-aarch64.tar.gz` (the same asset name as a tagged release, `tools/release.sh`).

Checked: in a local `debian:bullseye` container, `tools/linux-aarch64.sh setup` + `apt-get install zip` and then
`tools/portmaster.sh` on a stand-in binary (`/bin/ls`, the real cross-build wasn't run) builds
`dist/linux-aarch64/opentennis.zip`; every file the script reads is in git (`docs/screenshots/portmaster.png`,
`crates/hst/linux/portmaster/`, licences). The workflow YAML parses.

## Not verified / not 1:1
- The workflow itself hasn't run on GitHub yet (it runs on the next push to main): the artifact paths and beta upload
  are checked by reading only.
- `min_glibc` in CI comes from the real aarch64 binary; locally it came from the stand-in (2.28).

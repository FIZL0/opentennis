# T4 — CI gate before the beta

From the repo audit (2026-10-21-repo-audit, §3.1–3.2): `pages.yml` was the only workflow and published a beta on every
push to main with no checks.

## What changed

- `.github/workflows/ci.yml` (new): on pull requests, pushes to branches other than main, and `workflow_call`.
  - `test`: `cargo test --locked -p hst-sim -p hst-data`, then `cargo clippy --locked --all-targets -p hst-sim -p hst-data
    -p hst-oracle -p hst-gltf -- -D warnings` (the four crates without Bevy).
  - `deny`: cargo-deny 0.20.2's release binary (sha256-checked), `check` with `deny.toml`.
- `pages.yml`: a `ci` job calls ci.yml on pushes and manual builds. Releases and ship-local.sh's prebuilt deploys skip it.
  `beta` needs it to succeed; `deploy` needs it to succeed or be skipped. Every action is pinned to a commit SHA,
  keeping the same major version as before (checkout v4.4.0, rust-cache v2.9.3, upload-pages-artifact v3.0.1,
  upload/download-artifact v4.6.2/v4.3.0, deploy-pages v4.0.5). `dtolnay/rust-toolchain@stable` is replaced by
  `rustup toolchain install`, and the sniper container installs rustup with `--default-toolchain none` followed by the
  same command. binaryen's tarball is checked against the sha256 its release publishes (matched my download).
- `rust-toolchain.toml`: 1.99.0 (the stable everyone had), clippy + rustfmt, wasm32 + windows-gnu targets. I installed
  1.99.0 on the dev machine with all the targets the old stable had (android, ios, musl, i686, wasip1), so slots and
  scripts don't hit a missing target after the merge.
- `deny.toml`: licences allowed: the permissive set that occurs in the tree, plus MPL-2.0 (attohttpc). Advisories:
  unmaintained crates only fail when they are direct deps. Sources: crates.io plus FIZL0/ps2-float and ps2-oracle.
  Multiple versions and wildcards are allowed. cargo-deny counts our path and git deps as wildcards because the
  crates aren't `publish = false`.
- `Cargo.lock`: rustls 0.23.45→.46, rustls-webpki 0.103.15→.16, webpki-root-certs/webpki-roots 1.0.9→1.0.10 (all
  semver patches for RUSTSEC-2026-0346…0349, 0381…0383). Two advisories are ignored, with the reason written down:
  bincode unmaintained (RUSTSEC-2025-0141: no maintained release, ggrs' wire) and ruzstd 0.8 (RUSTSEC-2026-0344: pinned
  by bevy_image).
- `crates/hst-sim/tests/path_record.rs`: its two clippy warnings fixed (late init, `Case` type alias).

## Verified

- A fresh clone with no ISO and no `context/`: CI's test command passes (286 tests). The skip list covers the three
  that *fail* instead of skipping without recordings: `match_serve_walk`, `beach_plane_flies_over`,
  `serve_follow_through` (the T5 note says to drop the list once its helper lands). CI's clippy passes with 0 warnings,
  and `cargo deny check` gives "advisories ok, bans ok, licenses ok, sources ok".
- actionlint (latest) reports nothing on both workflows. `rustup toolchain install` with no arguments installs from
  the toolchain file (rustup 1.29).
- With fixtures: `path_record` still passes after the clippy fix.

## Not verified / not 1:1

- The workflows haven't run on GitHub; that only happens after the merge and push. The checks are actionlint and
  running the same commands locally. What could still differ: rustup's version on ubuntu-latest and in the sniper
  container (bare `toolchain install` needs rustup ≥ 1.28), and runtime on the runners (CI adds a Bevy-free
  build of hst-sim/hst-data, a few minutes; it runs in parallel with the builds).
- No fmt gate: `cargo fmt --all --check` gives 8,103 diffs and the task says to ask before reformatting →
  plan/HUMAN.md.
- Clippy only covers the Bevy-free crates: the Bevy crates have 13 warnings in files other tasks are editing → T9.
- The `sh.rustup.rs` installer in the sniper job is still fetched unpinned (it was before too; the task asked for
  action SHAs and binaryen).

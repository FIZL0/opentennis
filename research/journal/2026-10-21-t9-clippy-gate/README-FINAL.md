# T9 — workspace clippy gate (T4 follow-up)

T4's CI ran `clippy -D warnings` only on the four crates without Bevy. The Bevy crates (hst, hst-base, hst-play) had
13 warnings.

## What changed

- The 13 warnings are fixed. Everything except mips.rs is test code:
  - mips.rs: `chunks_exact(4)` → `as_chunks::<4>()`
  - net.rs: a tuple of two sends split into two statements
  - textures.rs: dropped the `0 +` from a test's expected mean
  - emote.rs: removed an `i32 as i32` cast and collapsed an `if let` + `if` into a let chain
  - home.rs: `RangeInclusive::contains`
  - transfers.rs: `Menu { screen, ..Default::default() }` instead of setting `screen` after `default()`
  - doubles_ai.rs and handover.rs: three `needless_range_loop` warnings, allowed on that loop with a comment. Each
    index addresses several parallel arrays (player/team) in a test, and a zip chain would read worse.
- `ci.yml`: clippy now runs `cargo clippy --locked --workspace --all-targets -- -D warnings`. Before it, it installs
  `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`: the system libraries Bevy's Linux setup lists. The
  lock has alsa-sys, libudev-sys and wayland-sys.
- `tools/done.sh`: the same command runs after the tests and replaces the old float-only clippy pass. The sim's
  `#![deny(clippy::float_arithmetic)]` modules are still covered, since a deny-level lint is an error either way, and
  the f32 hint is still printed when that lint fires. On failure it prints the warning/error lines (at most 30).

## Verified

- Locally (Rust 1.99.0 from rust-toolchain.toml), `cargo clippy --locked --workspace --all-targets -- -D warnings`
  exits 0. The only remaining output is cargo's manifest note (`main.rs` in several build targets), which isn't a
  lint and doesn't fail the run.
- The done.sh snippet was run alone: it passes on the tree. With a planted warning (the `0 +` put back) it prints
  `crates/hst-base/src/textures.rs:466:34: error: this operation has no effect …` and `CLIPPY FAILED`, exit 1.

## Not verified / not 1:1

- The CI job hasn't run on GitHub (it runs after merge and push), so the apt package list for ubuntu-latest is
  untested. Missing headers would show up as a `pkg-config` build-script error in the clippy step. actionlint isn't
  installed here, so the YAML edit wasn't linted (it's two plain `run:` steps).
- The done.sh clippy adds a check build of the whole workspace to every wrap-up, a few minutes with sccache.
  It doesn't take a `check.sh` slot, the same as the float-lint pass it replaces.
- Warnings that other branches land between this commit and their merge only show up in the runner's merge test
  (done.sh in their slot runs before this merge reaches them).

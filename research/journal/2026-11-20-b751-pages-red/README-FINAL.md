# B751 — CI: pages.yml red on main

Run 44833343118 (63286fc1) failed `ci / test` at clippy: `unnecessary_cast` on
`crates/hst/src/play/main_menu/continue_screen.rs:392` (`h.setup.sets as i32`, `h.setup.games as i32`; both are
already `i32`, added by B750). Dropped the two casts. `cargo clippy --workspace --all-targets -- -D warnings` (the
CI command) is clean locally.

## Not verified / not 1:1

- The CI run itself on GitHub: verified only by the same clippy command locally; the merge's CI run confirms it.

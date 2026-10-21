//! Test-only: the user's disc and the recordings in `context/` for the tests of every crate (on through the
//! `testing` feature). Neither is ever in git, so a test skips when its data is missing, saying so on stderr; a run
//! that has to check the port against the game sets `HST_REQUIRE_DISC=1` (tools/check.sh does) and then a missing
//! file fails the test instead of letting it pass without checking anything.
//!
//! The readers return what `std` would, so a test's `let Ok(..) = .. else { return }` stays as it is.
//!
//! A git worktree (a runner slot, `git worktree add`) has its own `context/`, often an old copy or none, and a
//! recording made in one stays there: a file missing here is taken from the main checkout's or another worktree's,
//! so every checkout checks against every recording on the machine. Recordings are kept in the main checkout's:
//! `tools/context_sync.py` copies a worktree's into it (run by `tools/pine.py` and `tools/done.sh`).

use crate::iso::Iso;
use std::io;
use std::path::{Path, PathBuf};

/// The disc image's file name, in the repo root.
pub const ISO: &str = "Hot Shots Tennis (USA).iso";

/// The repo root (for the disc, `context/` and `research/`).
pub fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// Every checkout of the repo that could hold `context/`: this one, then (when it is a git worktree, its `.git` a
/// file naming its git dir) the main checkout, then the other worktrees (a recording made in a runner slot stays in
/// that slot's `context/`).
fn checkouts() -> &'static [PathBuf] {
    static ALL: std::sync::OnceLock<Vec<PathBuf>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let here = root().canonicalize().unwrap_or_else(|_| root().to_path_buf());
        let mut all = vec![here.clone()];
        let common = std::fs::read_to_string(here.join(".git")).ok().and_then(|git| {
            let dir = PathBuf::from(git.strip_prefix("gitdir:")?.trim());
            dir.join(std::fs::read_to_string(dir.join("commondir")).ok()?.trim()).canonicalize().ok()
        });
        let common = common.unwrap_or_else(|| here.join(".git"));
        all.extend(common.parent().map(Path::to_path_buf));
        let mut trees: Vec<PathBuf> = std::fs::read_dir(common.join("worktrees"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|w| Some(PathBuf::from(std::fs::read_to_string(w.path().join("gitdir")).ok()?.trim()).parent()?.to_path_buf()))
            .collect();
        trees.sort();
        for t in trees {
            if !all.contains(&t) {
                all.push(t);
            }
        }
        all
    })
}

/// A path under the repo root (`"context/fixtures/x.bin"`; absolute paths are kept as they are). A `context/` file
/// this checkout lacks is taken from the first other checkout that has it.
pub fn path(rel: impl AsRef<Path>) -> PathBuf {
    let p = root().join(rel);
    if p.exists() {
        return p;
    }
    let s = p.to_string_lossy();
    let Some(at) = s.find("/context/") else { return p };
    if Path::new(&s[..at]).canonicalize().ok().as_ref() != checkouts().first() {
        return p;
    }
    checkouts().iter().map(|c| c.join(&s[at + 1..])).find(|q| q.exists()).unwrap_or(p)
}

/// Whether this run must have the data (`HST_REQUIRE_DISC=1`).
pub fn required() -> bool {
    std::env::var_os("HST_REQUIRE_DISC").is_some_and(|v| !v.is_empty() && v != "0")
}

/// A test that can't run without `what`: says it skipped, or fails under `HST_REQUIRE_DISC`.
/// `return fixture::skip("no recording")` in a test that returns `()`.
#[track_caller]
pub fn skip(what: impl std::fmt::Display) {
    assert!(!required(), "HST_REQUIRE_DISC: {what}");
    eprintln!("skipped: {what}");
}

#[track_caller]
fn checked<T>(r: io::Result<T>, p: &Path) -> io::Result<T> {
    if let Err(e) = &r
        && e.kind() == io::ErrorKind::NotFound
    {
        skip(format_args!("no {}", p.display()));
    }
    r
}

/// The user's disc.
#[track_caller]
pub fn iso() -> io::Result<Iso> {
    let p = path(ISO);
    checked(Iso::open(&p), &p)
}

/// `std::fs::read` of a file under the repo root.
#[track_caller]
pub fn read(rel: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let p = path(rel);
    checked(std::fs::read(&p), &p)
}

/// `std::fs::read_to_string` of a file under the repo root.
#[track_caller]
pub fn read_to_string(rel: impl AsRef<Path>) -> io::Result<String> {
    let p = path(rel);
    checked(std::fs::read_to_string(&p), &p)
}

/// Whether a file or directory under the repo root is there (skip/fail as above when it isn't).
#[track_caller]
pub fn exists(rel: impl AsRef<Path>) -> bool {
    let p = path(rel);
    let there = p.exists();
    if !there {
        skip(format_args!("no {}", p.display()));
    }
    there
}

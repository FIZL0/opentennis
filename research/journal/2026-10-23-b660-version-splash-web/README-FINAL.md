# B660 — version on the pre-ISO page and the splash

- Splash (`splash.rs` `build`): the main menu's version line (`home::version()`, now shared: release tag or `v` + crate
  version, ` - `, commit, `+` when dirty) drawn small (10, grey) in the window's top left, `Anchor::Sides`, under the
  quit popup. Top right is the New version card's corner; the bottom is the legend strip.
- Web page: `tools/web.sh` writes `web/build.txt` (git-ignored) = `${HST_VERSION:-v<Cargo.toml version>} - <commit>`;
  `index.html` fetches it (plain fetch, so sw.js serves it from the active build's cache like any page file) and shows
  it in `#build`, bottom right of the site header band (`site.css` `.band .build`). No file (a dev serve without
  web.sh) → nothing shown. `version.txt` stays the bare commit (the update poll checks its format).
- Checked: `HST_MENU=press tools/shot.sh` (label top left, readable); headless chromium of `web/` with a build.txt
  (label bottom right of the header). Shots in `context/b660/`.

## Not verified / not 1:1
- Front-end only, ours to design (PLAN.md): nothing to match in the original.
- Not seen on a phone or in portrait; the splash uses the same Sides pinning as the New version card and the page
  label is plain CSS in the header's wrap.
- The release web build runs web.sh without `HST_VERSION`, so the page shows `v` + the crate version, as the web
  game's own main menu does.

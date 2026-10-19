# B581 — new-version check on the splash and the launcher page

The checks themselves already existed (B317 browser `version.txt`, B374 desktop releases, B567 rechecks); only the
main menu showed them. This task adds the notice to the two screens a player sees first.

- Splash (`play/main_menu/splash.rs`): with a newer version out and 1P not yet taken, a New version card pinned to the
  window's top right: the version, the platform's button (Update installs in place where the self-updater can,
  Download opens the release page, Refresh reloads the browser page), "Settings and mods stay". △ (keyboard: the Lob
  key) or a tap on the button acts; the legend lists it. An install's progress bar and words replace the button.
- Launcher page (`web/index.html`, `site.js`, `site.css`): the version check fires an `hstlatest` event and is
  exposed as `hstCheck`; the site page asks at once and shows a New version panel above the play section with a
  Refresh button (drops the cached builds first when the service worker never finished the download, as the game's
  Refresh does).

See 01-FINAL.md.

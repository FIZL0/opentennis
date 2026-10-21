# M140 — update without reinstalling, on every platform

B374's updater replaced the game file in place, but only when the game's folder was writable. Phones needed a
reinstall. M140 finds the in-place route each platform's rules allow, adds a thin launcher for read-only installs,
and adds the Update? Always / Yes / No / Never prompt.

## What each platform allows, and the pick

| Platform / install | Rules | Pick (`update::Way`) |
|---|---|---|
| Linux/Windows, folder writable (tarball, zip, Deck non-Steam shortcut in ~/Games) | nothing in the way | **Swap**: download, check signature + sum, replace the file, restart (B374) |
| Windows Program Files, Linux system install (/usr, /opt) | the folder is read-only for the player, and asking for admin rights on every update is unfriendly | **Copy**: the new build goes to the user data folder (`update/<tag>/`); at start the installed game hands over to the newest copy that's newer than itself (`update::relaunch`, Unix `exec` / Windows wait + exit code) |
| Flatpak | `/app` is read-only, the sandbox can write `~/.var/app/<id>/data` | **Copy** (the data dir is the Flatpak's own) |
| Steam (in `steamapps/`) | Steam owns and verifies its library files; a changed file is "repaired" back | **Copy**, never touch Steam's folder; once Steam's own update overtakes the copy, the copy is pruned at start |
| Android, sideloaded APK | Allowed through `PackageInstaller` with `REQUEST_INSTALL_PACKAGES`; same signing key; the system always asks the user to confirm | **Apk**: download into app storage, PackageInstaller session, the system's confirm screen, status read back |
| Android, from Google Play | Play's Device and Network Abuse policy: an app from Play may not update itself by any route other than Play | **Page** (Play updates it) |
| iOS, App Store | Guideline 2.5.2: no downloading or running new executable code; iOS can't run downloaded native code anyway | **Page**: an App Store build gets updates from the store |
| iOS, sideloaded (SideStore / AltStore / TrollStore) | the installer re-signs and installs an .ipa from a link; the app may hand it the link | **Installer**: open `sidestore://install?url=…` (else altstore, then TrollStore's `apple-magnifier://`). The IPA lists those schemes in `LSApplicationQueriesSchemes` so `canOpenURL` can tell which is there. `tools/release.sh` also publishes `altstore-source.json` (AltStore v2 source format), so SideStore and AltStore can track updates on their own |
| Web | the service worker already loads the new build (B470) | unchanged |

Hot-loaded code instead of new binaries was rejected. The sim is native Rust, and shipping it as a script or wasm
module only for the platforms that forbid new binaries would mean a second runtime. On the one platform where that
would matter (App Store iOS), 2.5.2 forbids it anyway.

## Prompt and setting

- Settings → Updates (was Check for Updates) has Ask / Always / Never and is saved as `updates = ask|always|never`.
  Never is the old `update_check = off`, so older settings files still read correctly.
- Ask: the first time a run finds a version, the main menu asks Update? with Always / Yes / No / Never. It asks once
  per version per run. Always is saved and starts the install. Yes installs this one. No defers it for the run (the
  notice row stays). Never turns the checks off. ○ means No.
- Always: a found update installs without asking where the install finishes inside the game (Swap, Copy, Apk; the
  phone still shows its confirm screen). Elsewhere it only skips the prompt, and the notice row stays.

## Tests

`update::tests::copies_launch_the_newest_and_install_into_a_fresh_folder` checks the newest-copy pick, pruning, and
an install into an empty folder on both rename paths. `steam_owns_only_its_library` and
`installer_links_and_the_updates_word` are also new, as is `update_ask::tests::answers`. The settings round trip
covers the new field. The popup was screenshotted at 16:9 and at 8:3 (both in `context/m140/`) and scales correctly.

## Not verified / not 1:1

- Android: the PackageInstaller flow (session, confirm intent, status) compiles (Rust for aarch64-linux-android,
  javac for the activity) but hasn't been run on a device or emulator. Play-install detection hasn't been tried on a
  Play install either.
- iOS: the installer URL schemes and the `canOpenURL` check type-check but haven't been tried on a phone with
  SideStore, AltStore or TrollStore. The AltStore source JSON hasn't been loaded into AltStore or SideStore. Release
  IPAs need a recent `minOSVersion`; 15.0 is assumed.
- Flatpak/Steam: the Copy + relaunch path is tested on files only, not inside a real Flatpak sandbox or a Steam
  library. Whether a Flatpak can `exec` a binary from `~/.var/app` depends on the manifest's permissions (no
  `noexec` there by default).
- Windows: the relaunch (wait + exit code) and the Copy install are only checked with `cargo check` for
  x86_64-pc-windows-gnu, not run under Wine.
- Menus are ours by rule (PLAN.md): nothing to match against the original.

# M155a: the ARM Linux build as a PortMaster port

Followed https://portmaster.games/packaging.html, plus what PortMaster's own installer on the device
(harbourmaster 2606 on muOS: `captain.py` `check_port`, `harbour.py` `_install_port`) expects of a zip.

## What landed

- `crates/hst/linux/portmaster/`: the port's files.
  - `OpenTennis.sh`: the standard launch script: the template's `controlfolder` search (muOS's `control.txt` then
    sets its own, `/mnt/mmc/MUOS/PortMaster`), `source control.txt`, `mod_${CFW_NAME}.txt` if there, `get_controls`,
    `GAMEDIR=/$directory/ports/opentennis`, cd, `log.txt` overwritten each run (tee), `XDG_DATA_HOME=$GAMEDIR/conf`,
    `LD_LIBRARY_PATH=$GAMEDIR/libs.${DEVICE_ARCH}`, `SDL_GAMECONTROLLERCONFIG=$sdl_controllerconfig` (gilrs reads it),
    `WGPU_BACKEND=gl` unless set (as M154), `$ESUDO chmod +x`, `$GPTOKEYB "opentennis.${DEVICE_ARCH}" &` with no
    mapping (the game reads the pad itself; gptokeyb just gives the device's Start + Select quit, it kills with
    `pkill -f`, so the 18-character process name is fine), `pm_platform_helper`, run, `pm_finish`.
  - Disc: with no `disc.pack` yet (in `$GAMEDIR`, where the game keeps it beside itself, B668; or `conf/hst/` in
    case it ever falls back to the per-user folder) it passes `--import` the first `.iso` in `$GAMEDIR`, else a
    `*Hot*Shots*Tennis*.iso` in `/$directory/{ps2,PS2,roms/ps2,…,ROMS/PS2}` (ROCKNIX/Knulli's `roms/ps2`, muOS's
    `ROMS/…`), with a `pm_message` that the first start imports it. None: `pm_message` saying where the ISO goes,
    10 s, `pm_finish`, exit 1. An import that leaves no `disc.pack`: `pm_message` that the ISO couldn't be imported
    and that `log.txt` says why. `disc.pack` is the installed mark (the game writes it only on a good import, so no
    separate `.installed` file that could go stale if the pack were deleted).
  - `port.json` (version 4, `opentennis.zip`, items `OpenTennis.sh` + `opentennis/`, title/porter/desc/inst,
    genres `sports`, `rtr` false, arch `aarch64`, `min_glibc` filled in by the packager), `gameinfo.xml`
    (path `./OpenTennis.sh`, image `./opentennis/screenshot.png`), `README.md` (notes with the thank-you to Clap Hanz /
    SCE, controls from `play/controls.rs`' defaults, compile steps).
- `docs/screenshots/portmaster.png`: 960×720 (4:3) `--stage 1 --play` match frame of our renderer (shot.sh at
  800×600, `HST_WEATHER=0`), promo-screenshot exception.
- `tools/portmaster.sh` (run by `tools/build.sh linux-aarch64` after the cross build, on the host):
  `dist/linux-aarch64/opentennis/` in PortMaster's repository layout (port.json, README.md, screenshot.png,
  gameinfo.xml, OpenTennis.sh, `opentennis/` = `opentennis.aarch64`, `licenses/`, empty `libs.aarch64/`) and
  `dist/linux-aarch64/opentennis.zip`, the install zip: `OpenTennis.sh` on top, `opentennis/` holding the game,
  licences and the metadata (PortMaster reads `port.json`/`gameinfo.xml` from inside the port folder; its released
  zips are laid out the same way). `min_glibc` from `readelf --dyn-syms` (2.30 today). Licences: MIT, Apache-2.0,
  the menu font's OFL, CREDITS.md; no `.so` is bundled.
- `tools/release.sh`: `dist/release/opentennis.zip` (the zip is named after the port, as PortMaster wants), in
  SHA256SUMS and the upload, a *Handhelds with PortMaster* row in the download table. The plain ARM64 tarball
  (`OpenTennis/` + `crates/hst/linux/OpenTennis.sh`, for a Pi or any ARM Linux without PortMaster) stays.
- Release note line.
- Follow-up M155a1: the beta builds (pages.yml) carry the zip too.

## Checked

- `tools/build.sh linux-aarch64` from this worktree: builds, `opentennis.zip` lists as above.
- RG34XX SP, muOS 2606 (frontend stopped first, started again after): the old `/mnt/mmc/ports/OpenTennis` folder
  removed (its `disc.pack` kept), the zip installed with the device's own `harbourmaster install opentennis.zip`
  (twice: fresh and over itself): `OpenTennis.sh` → `/mnt/mmc/ROMS/Ports/`, `opentennis/` → `/mnt/mmc/ports/`,
  catalogue text and preview/box from the screenshot added, permissions fixed.
  - no ISO: `log.txt` has the no-disc message, exit 1, PortMaster's dialog process closed;
  - a fake `fake.iso`: launcher passes `--import`, the game says `not an ISO 9660 image`, the launcher's
    couldn't-import message follows, gptokeyb killed by `pm_finish`;
  - old `disc.pack` in `ports/opentennis/`: the game starts from it with no arguments, loads, and then panics in
    bevy_winit (no Wayland/X: M155's open windowing work), so the package is correct up to the window.

## Not verified / not 1:1

- The game itself doesn't run on muOS yet (M155: no display server), so gameplay, the pad through
  `SDL_GAMECONTROLLERCONFIG`, Start + Select quitting a running game and the import of a real ISO on the device were
  not seen. No real ISO fits on the device's card (1.6 GB free) for an on-device import test; the import itself was
  checked by M154/M155 earlier with the old launcher.
- The `pm_message` dialogs weren't seen on screen: with the frontend stopped, neither `/dev/fb0` (all zero) nor
  muOS's `mufbset -g` gave a picture; only the log lines and the dialog process starting/stopping were checked.
- Not tried on ROCKNIX, Knulli or ArkOS (no such device here); the ISO search paths for them follow their usual
  `roms/ps2`.
- No `cover.png` (optional: PortMaster uses the screenshot). The screenshot shows the keyboard legend (a desktop
  `--shot`), not the pad glyphs a handheld shows.
- The control table's face-button names (Anbernic/Xbox labels by position) are from the default bindings, not
  checked on the device.

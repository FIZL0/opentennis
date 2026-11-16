# M166 — other releases' discs beside the main disc

## What landed
- `hst_data::discs` (new): `Edition` (US SCUS-97610, JP SCPS-15113, EU SCES-54535) told apart by SYSTEM.CNF's boot
  line, then checked: the boot program and `ZZBIN/GAME.BIN` must be on the disc, and for US/JP the overlay must pass
  `exe::Game::new` (that release's 1.00 sizes/load address); EU only "is an MWo3 overlay" (no EU tables are read yet).
  `beside(main, dirs)` looks through folders (not recursive) for files ≥ 32 MB under any name (links followed, `.part`
  skipped), opens each as ISO or pack, leaves the main disc out, one disc per release. `main_first` prefers the US
  disc among several images (iOS `disc::first_iso`, Android's start when there is no `disc.pack`).
- `hst_base::releases` (new): the registry the Rules setting (M175) and crossplay (M171) read: `have()` (main first,
  then siblings), `get/has/disc_of(Edition)`, `changes()` counter for menus. Each entry carries the path and the
  gameplay digest (`hst_data::digest`; main = `handshake::sim()`, JP computed; EU `None`). `start(main, voices)` runs
  `find` on a thread at startup (hooked in `hst` `start()` beside `handshake::set_disc`, and in iOS no-disc mode).
  Scans the main disc's folder and `voices/`.
- `voices::import_disc(path, dir)`: one import at a time (a lock), skips a release whose pack exists or that brings no
  voices; `import_waiting` now goes through it, so the start's voices/ scan and the sibling import never write the same
  pack at once. Sibling JP/EU discs' voices are read into `voices/` automatically.
- Tests: `hst-data discs::tests` (boot lines; finds JP/EU in `fixtures/discs/`, renamed link found, small file and
  duplicate release passed over, main_first), `hst-base releases::tests::registers_the_discs_beside_and_their_voices`
  (US main + linked JP/EU siblings: order, paths, JP digest ≠ US, EU none, both voice packs written, second import
  does nothing). ~2 s.

## Phones / web
- Android/iOS: the main disc's folder is the app's own folder (Android external files dir, iOS Documents), so discs
  put there (Files app / adb) or in `voices/` are found at start. Their pickers take one file, which Add Voices (M165)
  reads in for voices only; a picked disc is not kept, so it is not registered for gameplay.
- Web: no folder to scan; only the main disc is registered (without a digest).

## Not verified / not 1:1
- Not run on a phone or in the browser build (no device in an unattended run); the wasm branch of `releases::start`
  and the Android/iOS `main_first` lines were only compiled for desktop.
- Raw 2352-byte-sector images (.bin/.img) aren't recognised: the ISO reader takes 2048-byte ISO 9660 images and packs.
- Nothing consumes the registry yet: the Rules setting is M175, streaming to peers M171.
- A disc picked through Add Voices on phones isn't kept as a gameplay sibling (would need copying ~170 MB into a pack).
- Startup plumbing only: no behaviour of the original involved, so nothing to check against PCSX2.

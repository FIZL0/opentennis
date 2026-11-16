# M165 — Voice Language: add a disc in-game

Settings → Sound has a new last row, **Add Voices** (under Voice Language; not in the browser build).

- ✕ / tap calls `hst_base::voices::add_disc()`: a worker thread opens the platform's picker, reads the disc's voices
  into `voices/<disc id>.pack` with `voices::import_with` (new: `import` plus a progress callback, by the sound
  archives' bytes read), and publishes its state in `voices::adding()` (`Idle`, `Picking`, `Reading(f)`, `Added(langs)`,
  `Failed(why)`). The pack cache is dropped at the end, so the new languages are on Voice Language at once.
- Pickers: desktop = zenity/kdialog first on Linux (Steam Deck Gaming Mode), else rfd (as the first-start disc picker);
  Android = the activity's `ACTION_OPEN_DOCUMENT` picker (`android::pick`/`taken`, polled from the worker) read through
  the file descriptor (no copy); iOS has no picker on main yet, so the button reads the ISOs already put in the voices
  folder through the Files app, right away instead of at the next start.
- The row's value says what's going on ("Choose disc" / "Choosing…" / "Reading 42 %" / "Added" / "Not added"), the side
  panel's note the details (the languages added, or why not: the US disc and other discs get a plain-English refusal
  before anything is written). The legend says "Add disc". A second press while busy does nothing.
- A release imported before is read again (the pack is replaced atomically via `.part` + rename).
- Under tests (`testing` feature) the desktop picker returns nothing, so the every-row tests never pop a dialog.

Tests: `voices::tests::add_voices_reads_a_disc_with_progress` (US disc refused, nothing written; Japanese disc read
into its pack, progress monotonic 0 → 1, `adding()` ends at `Reading(1.0)`), `options::tests::add_voices_row` (row under
Voice Language, never picked, ◀ ▶ inert, ✕ clicks and starts/ends with nothing picked), legend test updated.
Screenshots of the page: `context/m165/sound_end.png`, `context/m165/sel.png` (1280×720).

## Not verified / not 1:1

- Front-end only, no original to match (PLAN: menus are ours).
- The real pickers weren't opened: an unattended run mustn't pop a dialog on the user's desktop, and there is no
  Android device/emulator run here. The Android path reuses the start page's picker calls from a worker thread
  (JNI attaches it); that the native activity survives the picker going over the running game (pause/resume) is
  untested. macOS: rfd's dialog runs off the main thread (same caveat as Add mod's).
- iOS: no in-game picker (it lives on concept/ios-file-picker); the button reads the voices folder instead.

# B601: M136's mod transfer pill

M136 (587a6689) wasn't on main when this started (its merge, daf53c36, was still in the runner), so its crate changes
were applied onto task/B601 first (separate commit) and fixed on top. When both branches merge, the hunks are the same.

## Driven for real (remaster, `tools/shot.sh` + `HST_KEYS`, a disc folder with an empty `mods/` each run)

Main → Mods → Browse, five character installs queued (Downloads at once: 3), Esc to Main, ↓ ✕ into Settings, shots
every few seconds until the batch ends (`context/b601/run3`, `run4`, `run6`).

Found:
1. **The bar never filled.** The default install (no upscaled set) is the ranged one, whose progress words were
   "Downloading X... N MB", with no total, so the pill had no percent to read. The ranged path now learns once it
   has read the zip's directory how many bytes the kept files need (`modfs::unpack_planned` / `install_planned`; the old
   functions call them with no plan) and words it like the whole-zip download: "N / M MB (P%)". A lobby's looks
   fetch (`fetch_looks`) gets the same total.
2. **The count stuck at "1/3"** with three downloading side by side, until all three were in. Now it shows the
   range under way: "1-3/5: Alex +2", then "3-5/5: Bjorn +2", "4-5/5: Bloom +1".
3. **Uploads** (from reading `modstore::publish`): the store packs a batch's mods one after another, marking each one
   under way as it starts, and they all stay under way until the batch's one commit lands. So the pill named any of
   them (hash-map order) and its bar sat at 0 until everything finished at once. It now names the one started last
   ("b +1"), shows the range, and counts a mod under way as half done, so the bar moves while packing.
4. **The cursor moved on other screens.** When downloads end, the store listing is read again. When it landed,
   `listed()` moved Browse's cursor to the first tile, but it wrote the shared `menu.sel` while Settings was up
   (Display jumped to Button Icons, and later to Key Hints). Off the Mods screen it now moves a scratch cursor.

Worked already: pill top-middle on Mods, Main and Settings (and on any screen with `HST_TRANSFERS=1`); it clears when
the batch ends; the Reload / Later popup then comes up over Settings (a calm screen) or Main.

Tests: `transfers::tests::downloads_side_by_side_show_their_places`, `an_upload_batch_as_the_store_sends_it`,
`mods_screen::tests::a_listing_elsewhere_keeps_the_cursor` (fails without the fix: cursor 12), and
`modfs::tests::plans_the_bytes_it_reads`.

## Not verified / not 1:1
- **No real upload.** This machine has no Hugging Face write token, and an upload would publish to the public store.
  Upload behaviour is checked by a test that replays `publish`'s event order instead. Someone with a token should
  upload two mods once and watch the pill.
- Only character downloads from Browse were driven, with one queue size (5, three at once), starting from the Mods
  screen and then on Main and Settings. The select, lobby, Online and Extras screens only had the `HST_TRANSFERS`
  sample.
- The bar pauses for a moment while each mod is checked and unpacked after its last byte arrives (no progress
  words for that step).
- Menus are ours, so no comparison with the original.

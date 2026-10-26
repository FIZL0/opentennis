# M149 — each input's last pick

Front-end work (ours to design, no PINE proof needed).

## What landed
- `crates/hst/src/play/main_menu/last_picks.rs`: per-input last pick (Random or a character by its favorites key,
  costume, hand toggle), keyed by the input, not the seat: `keys` for the keyboard, `pad <vid>:<pid> <n>` for a pad
  (`n` = its place among connected pads of the same model, lowest entity first). Saved to `picks.txt` beside
  `settings.txt` (also in `store::SAVED`, so the B668 migration carries it).
- `last_picks::track` (before `step`) keeps the pad keys current, notes every input ready on its own seat and
  writes the file when a pick changes (not under `HST_MENU`, so `--shot`s don't write).
- Recalled where a select hands out fresh players: `local_start` (1P), the assignment's way on (`assign_leave`),
  `online_enter` (Quickplay/Host/Find/Code), a ✕ join on the local roster, `lobby_join`.
- Fallbacks: a mod that's gone (or a mod while the lobby has mods off) → the default pick (the Random card), hand
  kept; a costume past the character's count (a costume pack removed) → its first costume; an input with nothing
  saved → as before. Computer seats untouched (B269's `Menu::last`).
- Tests (`last_picks::tests`): keys for two pads of one model, file round trip, local match → back → next select
  (each input on its pick on whatever seat it gets), online enter + lobby join, fallbacks, CPU seats left alone.

## Not verified / not 1:1
- Not tried live in the app (unattended run; menu behaviour is covered by the unit tests on `Menu`). The original
  game has no such feature, so there is nothing to compare against.
- Two pads of the same model are told apart only by connection order (entity order); if they reconnect in the
  other order after a restart they swap picks. No stable per-device serial is available through Bevy's `Gamepad`.
- A mod pick recalled on `online_enter` happens before the room is known, so in a mods-off lobby it is undone by
  the existing `unmod` only once readied (as any mod pick is).

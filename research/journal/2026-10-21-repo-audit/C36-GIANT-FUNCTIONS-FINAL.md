# C36 — the giant functions split

The seven functions the audit flagged are split along their existing phases. The code is moved, not rewritten.
Statements keep their order, so the court RNG draws, the Commands order and the sim arithmetic stay the same.

| function | before | after | split into |
|---|---|---|---|
| `main_menu::online` `online_step` | 789 | 38 | `online_menu_step`, `lobbies_step`, `code_step`, `lobby_step` → `lobby_press`; the ↑↓ wrap is the shared `step_rows` |
| `hst-base::net` `host` | 622 | 77 | `roll_code`, `deliver`, `struct Hosted` holds the room loop's state, with one method per `select!` arm (`settle`, `list`, `start`, `back`, `on_guest`, `ping`, `count_down`, `wait_down`, `home`, `rolled`, `on_cmd`) |
| `main_menu` `step` | 557 | 248 | `main_step`, `controls_step`, `assign_step`, `chars_step`, `venue_step`, `confirm_rows_step`, `step_rows` |
| `play` `setup` | 381 | 96 | `new_game`, `disc_character`, `load_player`, `match_sounds`, `load_effects`, `spawn_ball`, `spawn_balloons` |
| `play::npcs` `step` | 367 | 127 | `point_news`, `walk`, `figures`, `at` (each part makes its own `roll` on `g.rng.court`, in the same order) |
| `main` `load` | 345 | 78 | `load_court` (181), `court_library` (`CourtLibrary`), `match_weather`, `net_decal`, `placed_box` (the B303 and B351 boxes used the same expression), `spawn_parts`, `add_parts` |
| `hst-sim::npc` `callback` | 349 | 12 (dispatch) | `wildlife`, `obstacles`, `props`, `court_shows`, `gallery`, grouped by type; each one keeps its `(type, message)` arms in their original order |

In `callback`, each family's types are disjoint, so a guard that fails still falls through to `_ => {}` as it
did before. The only arm that moved, `(44, GAME_END)`, goes with type 44's other arms. No other arm for type 44
matches GAME_END, so the move changes nothing.

Tests: `hst-sim` npc (5) and replay (4), `hst-play` npcs (7), setup (2) and netplay (35, `HST_NETPLAY=1`),
`hst` main_menu (256) and `hst-base` net (44) passed. `hst-base` also builds for wasm32. The full suite and clippy
run in `done.sh`.

Still over 150 lines, all outside this task: `main_menu` `setup`/`draw`, `online` `layout`/`sample`, `play`
`strike`/`serve_turn`/`advance_stroke`/`simulate`, `main` `parse`/`start`, `npcs` `setup`. `net`'s `on_guest`
(154) is one dispatch over guest messages.

## Not verified / not 1:1

none. This is a refactor with no behaviour change, checked by the replay, menu, netplay and npc tests. Nothing
was checked against the original game because no gameplay changed.

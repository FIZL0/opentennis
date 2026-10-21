# 26 — P116: AI accuracy review

Every AI decision of the original, where the port has it and what proves it. "Rec" is a hooked PINE recording replayed
call by call (each call fed the AI generator as it stood, its outputs and draw count compared bit for bit); "oracle"
is `hst-oracle` over ≥10k generated inputs. Fixtures are in `context/fixtures/` (git-ignored).

## New in this task

- **Bot second serves and the singles serve-and-dash aim**, the two serve cases no recording had (from HUMAN.md's
  old P11i item). `tools/record_ai_serve.py` takes `HST_AI=k:row:level` (as `record_ai_rally.py`) and
  `HST_SERVE_ERROR=<frames>`. The second one pokes every CPU row's serve timing error, a cell the hooked calls don't
  read, so the replay against the disc table still holds. With it the bots fault, and second serves follow.
  - `ai_serve_singles_dash.bin`: `HST_AI=0:49:3 HST_SERVE_ERROR=25 tools/pcsx2.sh python3 tools/record_ai_serve.py 8 9000 …`
    from `bots_singles.p2m2`'s state copied to copy 5's slot 8. P0 is on row 49 (ALL, net dash 70 %). Result: 14
    tosses (2 on a second serve), 13 aims, 5 serve dashes (the net-dash rate and the level-2 follow), plus 56
    button locks and 44 stick locks.
  - `ai_serve_s05_second.bin`: `HST_SERVE_ERROR=25 … record_ai_serve.py 5 9000`. Result: 11 tosses (1 on a second
    serve), 11 swings, 10 aims.
  - `ai_serve.rs` `singles_second_serves_and_net_dash` and `doubles_second_serves` pass bit-exact.
- **Change of ends** (measured on copy 5, slot 5 doubles; the script polls the AI generator's index and the match
  phase each frame). Phase 1 (81 frames, the app's `CHANGE_ENDS` 80) draws 87 times. The serve phase after it draws
  64, against ~82 after an ordinary point. So the original draws about 69 more times per change of ends than per
  ordinary point, about one round of four AI resets. The app's `doubles_ai::step` returns at once in ChangeEnds, and
  only bot's leftover stand-in walk runs there. → **P127**.
- Two stale comments fixed: `ai_update`'s doc said the routines were still the stand-in's. The zeroed stick quad in
  `doubles_ai::step` is the game's own behaviour (P11k6 spec), not a ponytail.

## Inventory

| Decision | Port | Proof |
|---|---|---|
| AIParam.csv parse | `hst_sim::ai::AiParams::table`/`bytes` | `ai.rs table_equals_ram`: all 168 rows byte for byte vs RAM |
| Row by character + outfit, singles/doubles half | `ai::menu_row`, `Choice::new`; app `ai_params`, roster/menu | `bots_get_their_rows` (slot 5 RAM), `rows_by_outfit_and_mode` (unit) |
| Difficulty (level) | `Choice::new`: level 3 unless the setup forces 1..4 | Exhibition is always 3 (the game's six-entry table, all 3), matching slot 5's RAM. The forced levels (challenge setup byte) are unit-tested only, and the app has no mode that forces them |
| Level reach (2.5/+0.35/+0.7) | `Choice::reach`; the app passes level 3's | Slot 5 RAM (`bots_get_their_rows`); exhibition only |
| AI generator seed and set-up draws | `hst_sim::mt`, `play/ai_setup.rs` | MT reference outputs; `ai_setup::tests`: generator index at setup and after point resets vs captures (singles human/CPU, CPU/CPU, doubles) |
| Mind: point reset, roles, hand-over, point-over coin, ALL net rate | `ai::Mind`; `play.rs ai_update` | `minds_match_the_game` (ai_mind_s05); `ai_rally.rs glue_matches_the_game` (5135 updates/messages) |
| Mind across a change of ends | none (stand-in walk) | **Gap P127**: the original draws ~69 extra times |
| Timing error (stroke/volley/smash/serve, pace) | `AiParams::timing`, `picks` | `timing_errors_match_the_game`; picks on 155 draws (ai_s05) |
| Reaction delay | `AiParams::reaction` | `reactions_match_the_game` (ai_pos_s05); view gate P113/P115 |
| Guessing (and verdicts) | `AiParams::guess`, `Guess::verdict` | `guesses_match_the_game`, `guess_verdicts_match_the_game` |
| Serve spot and wait | `ai::serve_spot`, `serve_wait`; `bot_serve` | `minds_match_the_game` |
| Serve toss (first/second, underhand) | `AiParams::serve_toss` (aim.rs) | `ai_serve.rs`: first serves (both fixtures); **second serves new here** |
| Serve swing kind | `serve_swing` | `ai_serve.rs` |
| Serve aim (doubles; singles + net dash) | `serve_aim`, `serve_aim_singles` | `ai_serve.rs`; **dashes new here** (5) |
| Serve contact pick (quick serve) | `serve::ai_pick` | `ai_serve.rs` tag 4 |
| Serve aim frame (frame before the contact) | `bot_serve` / `ai_serve_aim` | At the press when the app's serve search puts the contact one frame away. **Gap P130** |
| Serve → rally hand-over draw order | `ai_update` | The app draws the net repick before the Return roll, the game the other way round (P11k6 note). **Gap P130** |
| Receive (doubles, singles) | `hst_sim::rally` receive | `receive_matches_the_game(_long)`, `vpad_serves_receive…`, `singles_receive…`, `singles_vpad_serves_receive…` |
| NET / BASE rally routines | `rally.rs` | `net_and_base…`, `singles_net_and_base…`, `vpad_rallies_net_and_base…`, `singles_vpad_net_and_base…`, `rally_state_tick_by_tick_on_match_s05` |
| Contact searches (reach, tier, last entry, smash find) | `ai::reach_search`, `tier_search`, `rally.rs` | `ai_search_*` fixtures. Last entry and smash find are reached only on poked states (journals 23/24) |
| Run estimate | `ai::frames_to` | oracle `run_frames`, `run_frames_handicapped` (20k each) |
| Target (doubles pair aim; singles rally/receive aim) | `hst_sim::aim` | `aims_match_the_game`, `singles_aims_match_the_game`. Out-of-court aims only with a forced row (journal 24-L4) |
| Shot kind, kind lock (singles) | `AiParams::picks`, `aim::button`, `lock_button`/`lock_stick` | `ai_serve.rs` (65 + 56 button, 57 + 44 stick locks) |
| Lob vs a net rusher | rally routines' depth-zone pick | `net_rush_matches_the_game` (P111) |
| Smash (taken, timing) | rally routines; timing smash error | BASE smash taken (`ai_rally_singles_base`); `ai_rally_vpad_smash`, `_singles_vpad_smash`. Aim margins/deep mode: open **P117** |
| Dive | `swing::dive`, drawn chance | `ai_dives_match_the_game` |
| Let-go (out balls) | `aim::lets_go`, routines' line-margin hold | `cpus_let_out_balls_go` (B57); the hold only with poked margins |
| Positioning: doubles formation | `hst_sim::position::Formation` | `formation_matches_the_game`, `formation_pick_matches_the_game` |
| Positioning: singles centre, net dash | `position::Single`, `Rally::dash_spot` | `singles_matches_the_game` |
| Strong-side run-round | `run_round`, `stand_side` | `stand_sides_match_the_game` |
| Partner split / claim (doubles) | `play/doubles_ai/handover.rs`, give-way | P11n2: match_s05 (the same player goes); `give_way_test` in `vpad_rallies…`; the claim exemption is a rule test only |
| Call-outs (give-way voice) | `Rally::said`, `sound::call_out` | P11n2 scan of 11 doubles fixtures (gate armed/cleared) |
| Human partner's records (CPU beside a human) | `doubles_ai::human` (P11k7) | `human_record_matches_the_game` |
| Hit message (doubles: repick, re-entry) | `Rally::heard_hit_doubles` | `glue_matches_the_game` |
| Hit message (singles), shot target, under_min | `doubles_ai.rs` | **Open: P11l6** (spec written, port pending) |
| Strike (0x14) / new path (0x16) messages | `Rally::heard_strike` | Spec only, no recording. **Gap P129** |
| The app's `doubles_ai::step` wiring as a whole | `play/doubles_ai.rs` | Only the sim glue replays; the app's step is checked by its playing tests. **Gap P128** (with P11l6 for singles) |
| Path object (mode, gap, window) | `hst_sim::path` (P11n1) | Per-frame record test; `path_mode_and_gap` |

## Stand-ins left (ponytail: / stand-in)

- `play.rs bot` tail (follow-through, whiff, approach, `ai_wait`/`ai_wait_singles`, `bot_run`) runs only where
  `step` returns false: the change of ends (P127) and player counts other than 2/4.
- `ai_aim`, the fallback when no routine left a press, and `ai_serve_step`'s goal (B27d) go to P128.
- `play.rs:1310` (who is human is taken from the pads at setup) needs no task: sides are fixed at the start.
- `ai.rs` stand_side (the entry before the path), frames_to (the unset speed register) and tier_search (the unset
  band past tier 3) are uninitialised reads in the original. They can't be matched, and they are rare.
- `doubles_ai.rs` 232 and 367 (the singles hit spot, the shot target) are P11l6. 402 (cleared at the point's set-up)
  is where the serve message lands.
- The NET/BASE stand scratch (`Rally::stash`) is an uninitialised read in the game (P11k6 note). It can't be matched.

## Mod characters

Same code paths: `mod_match::apply` gives a mod player `ai_params(m.ai_row, outfit)` (its donor's row by the slot's
costume) and `ai_second_toss(m.ai_row)`, and the reach, timing (`restat`), smash heights and aim stats from its own
TParam row. The doubles smash offset comes from the mod's own skeleton (`smash_side`). Bots run the same `bot` →
`doubles_ai::step` → `hst_sim::rally` routines for mod and disc characters; nothing branches on a mod.

## Not verified / not 1:1

- The new serve fixtures run on poked rows: the serve timing error (all CPUs) and P0's row 49 in singles. The
  serve-error cell feeds the timing draw before the serve, not the hooked calls, so the replayed calls are the
  game's own on the disc rows. The second serves come from bots made to fault, not from the stock rows.
- The change-of-ends draw counts come from one change of ends in one match (slot 5 doubles), by polling the
  generator index once per frame, not by hooking it. Which calls make those draws isn't known yet. Singles wasn't
  measured. The app's own draw count there wasn't measured either (by reading the code: a Return re-roll at most).
  All of this is P127.
- The difficulty levels 0–2 forced by a challenge setup are unit-checked only. The app has no challenge mode.
- Branches reached only on poked states (last-entry pick, smash find, 60-frame redraw, chase arrival, line-margin
  hold, short-landing walk past the window) stay as journals 23–25 left them: the code is bit-exact on recorded
  inputs, but no unpoked recording reaches them.
- Rates over whole matches (e.g. lob share vs a net rusher, P111) are compared by scenario, not frame by frame. The
  app can't load a PCSX2 state (P128 would let it).

# M176 — Match Speed plumbing + setting

## What landed
- `hst-base/src/match_speed.rs` (new): the match's ticks a second.
  - `current()`: the running match's rate. Each match's flags set it (`--match-hz N`, 60 without the flag; parsed in
    `main.rs` beside `--no-shot-markers`).
  - `MatchHz`: the same number as a resource. `play::plugin` inserts it with `Time<Fixed>` built from it (at 60 it is
    the same `from_hz(60.0)` as before).
  - Setting: `match_speed = 60|50` in settings.txt, `choose`/`chosen`/`available`/`name`.
  - `playable(50)` needs the European data here (`releases::has(Europe)`, or the main disc is EU) **and**
    `FIFTY_HZ_SIM`. That const is `false` until M169a1 puts the European program's 50 Hz timing in the sim. Without
    it a 50 Hz tick plays the US sim 5/6 as fast, which is not the original, so the menu doesn't offer it yet.
  - `name(60)` on an EU disc alone (no other release laid by Rules) is "60 Hz (close, not the original)": the
    M170 note.
- Uses of the rate:
  - **Fixed clock:** the match plugin; the court intro's own accumulator and its hand-over (`intro.rs`); the pause
    screen's two restores (`hst/src/play/menu.rs`).
  - Left at 60 on purpose (menu clock, not the match's): `mode.rs` teardown (menus), `court_anim` (base app; the
    match plugin overrides it at mode start), and the previews/inspect/sandbox.
  - **Character drawing:** `character.rs` `animate` turns fixed-clock seconds into ticks at `MatchHz` (60 when there
    is none, i.e. in the menus). That covers serve-ready, ease-in and its debug print.
  - Figures, ball, camera, markers and the rest already blend by `overstep_fraction`, which works at any rate. So
    every moving thing interpolates at 50 Hz on any display.
  - **Match statistics:** the match time uses `hst_sim::stats::hms_at(frames, hz)`. `hms` itself is unchanged.
  - **Online:**
    - `Room::hz` is the host's Match Speed, 0 meaning 60. It is skipped on the wire at 60, so a 60 Hz lobby's JSON
      is byte-identical.
    - `netplay::flags` passes it as `--match-hz`.
    - The rollback session runs `with_fps(room.match_hz())` (ggrs' default is 60, so nothing changes at 60).
    - The host's input delay is `delay_at(rtt, hz)`; `delay_for` = `delay_at(rtt, 60)`.
    - A guest that can't play the room's speed is turned away with a reason (`match_speed::refused`) on the first
      `Room`/`Start`.
    - `recover::tick` at run time already takes real seconds, so it needed no change. The `1/60` in netplay.rs is
      test-harness code, where rounds are 60 Hz ticks.
- Menu:
  - Settings → Gameplay → Match Speed, after Rules. It shows only when there is a choice
    (`available().len() > 1`), so today it is hidden.
  - Offline matches get `--match-hz` from the main menu's match flags. Cancel restores the setting, and settings
    round-trip.

## Tests
- `hst-base match_speed::tests` (setting words, flags, 50 waits for the sim, 60 Hz clock unchanged).
- `hst-sim stats` (`hms_at`).
- `hst-play netplay::tests::flags_carry_the_match_speed` (flags, wire omits hz at 60, delay in ticks).
- `hst options::match_speed_row`.
- `main_menu settings_round_trip`.
- `HST_NETPLAY=1 tools/check.sh -p hst-play netplay`: 40 passed (rollback session built with `with_fps(60)`).
- Smoke: `--play --pads -,-,-,- --match-hz 50` bot match on court 1 runs and draws (`context/m176/hz50.png`).

## For M169a1
- Flip `match_speed::FIFTY_HZ_SIM` once the sim plays 50 Hz. The row then shows by itself where EU data is here.
- Take the rate from `match_speed::current()` / the `MatchHz` resource (both are 60 in every existing test).
- Sim-side ×60s that stay as they are until then: km/h ×60, `flow.rs` hold seconds, AI serve wait, `LONG_RALLY`,
  the BGM clock.
- A 50 Hz match must read the EU disc's raw TRAJ, not M170's 60 Hz regeneration.

## Not verified / not 1:1
- **50 Hz gameplay is not the original yet:** the sim's 50 Hz switches are M169a1's. Until then 50 Hz is not
  offered (`FIFTY_HZ_SIM = false`); only `--match-hz 50` on the command line reaches it, and it plays the US sim
  slower.
- **Rendering at 50 Hz on a 60/120 Hz display was checked by code reading only** (everything blends by the fixed
  clock's overstep), plus one still shot. Smoothness wasn't watched live: this was an unattended run, and the user
  tests UI live.
- **Character drawing constants in ticks** (ease lengths, `SERVE_READY`) count ticks of the running rate. At 50 Hz
  with M169a1's motion rescale they may want rescaling. This is presentation only; check by eye once 50 Hz plays.
- **Court life:** court animation (`court_anim`), weather and the like tick per fixed step. They follow the match
  rate but keep their 60 Hz per-tick amounts; the EU retimes of these are part of M169a1's spec.
- **Guests and EU data:** a guest without EU data is refused rather than streamed the data (M171 streaming of the
  50 Hz data needs M169a1's data list). The lobby list doesn't show a room's speed.
- **Rollback constants** (`WINDOW`, `SPREAD`, `UNACKED`, `HASH_EVERY`) stay in ticks. At 50 Hz they cover 20% more
  time, which is harmless; the netplay match tests weren't run at 50 Hz (there is no 50 Hz sim to keep in sync).
- No original behaviour involved, so nothing was checked against PCSX2.
- Tooling: this branch's base lacked main's one-line `#[allow(dead_code)]` in `third_party/bevy_transform` (from T23), so the web check failed on warnings not ours; the identical line is applied here so done.sh's web check passes and the merge is clean.

# M163a1a — the Japanese disc: checked in PCSX2, then opened

The Japanese disc booted in PCSX2. I drove its menus to a CPU-vs-CPU match and checked the parts of the Japanese
build that can be checked without a frame-by-frame recorder. Then `Release::playable` was opened for Japan and
the front-end's reads from the menu archive were made to fall back.

## The run (scratch: `context/m163a1a/`; states kept in `fixtures/m163a1a/`)
- `tools/pcsx2-hst.sh` takes `HST_ISO=<path>` to boot another disc (the default is still the US ISO):
  `HST_ISO=fixtures/discs/SCPS-15113.iso tools/pcsx2-hst.sh`.
- The menu path from boot:
  - Start, then the memory-card check.
  - Play data 1 (an existing Japanese save on copy 3's card), with ○ to confirm.
  - Main menu → Right to みんなでテニス → ○.
  - シングルス → ○.
  - First server: Down ×9 to COM → ○. Second: Down ×7 to COM → ○.
  - Characters: ○ (エレノア / Eleanor), then the second CPU's pick (ジュン / Jun).
  - 試合開始 → ○. The court is Aloha Beach (clay), umpire Chika, 1 set of 4 games.
- Saved states:
  - slot 8 at the court intro: `fixtures/m163a1a/jp-bot-intro.p2s`
  - slot 9 mid-match: `fixtures/m163a1a/jp-bot-match.p2s`
  - Slots 8/9 are scratch, so the copies in `fixtures/` are the ones to keep. Load one by copying it to
    `SCPS-15113 (A35F68F7).0N.p2s` in the copy's `sstates/`.
- The RAM image for the oracle is `fixtures/ram/jp-match.bin`, taken from the mid-match state with
  `7z e -so … eeMemory.bin`.

## What was checked
- **km/h (SPEC §6), bit-exact.** `crates/hst-oracle/tests/japan.rs::japanese_kmh` runs both of the Japanese
  program's km/h routines on its own RAM image: the ball's last move, and the vector the other callers pass. Each
  gets 20k inputs, compared with `Build::Japan.ball_kmh`.
  - Every result matches to the bit.
  - The US formula misses on some of those inputs, so the test can tell the two builds apart.
  - Both routines add the squares in y, x, z order.
  - The addresses are under `jp_*` in `research/oracle.txt`.
  - The test skips without the JP ISO or the RAM image, even under `HST_REQUIRE_DISC`.
- **Red landing marker on the serve (SPEC §5), by eye.**
  - A burst of PCSX2 screenshots in the CPU-vs-CPU match shows it at the serve's aim point in the receiver's
    service box: on a fault ("フォルト"), and on the 99 km/h second serve that followed.
  - It shows from the moment the serve is hit, as the spec says.
  - Our headless test `build_rules::japan::a_bot_match_on_the_japanese_disc` checks the same thing on our side.
- **The JP match plays.** It ran in PCSX2 through several points, with faults and rallies, and the speed readout
  in km/h.
- **The remaster on the JP disc** (`target/release/hst <jp.iso>`, with `HST_DATA` in a scratch folder):
  - A doubles CPU match ran 60 s with no panic.
  - The pause screen opened with its venue caption.
  - These menu screens all open: main, mode, chars, confirm, confirm-court, confirm-handicap, settings, sound,
    graphics, controls, assign, extras, mods, online-custom, roster2-favs, lobby-bot, names.

## What changed
- `hst_data::exe::Release::playable` accepts Japan. The disc import's wrong-disc message names both discs.
- `main_menu/disc_text.rs` reads the menus' text table. The Japanese disc has no `message0.dat` (its menu text is
  in Japanese inside its program, and our letter sheet can't draw it). For that disc it fills our own words:
  - the Local and Settings rows' descriptions
  - the mode screen's line
  - "Court N" and "Umpire N" for the court and umpire names
- `main_menu.rs` and the pause screen's venue caption and controller-assignment screen read the text through it.
  They no longer stop on a disc without the table.
- The main menu's header shows the disc's own serial (SCUS-97610 or SCPS-15113) instead of a fixed US one.
- The other moved art in the spec (§8) was already handled in M163a1 (`play/release_art.rs`). The menu sheets,
  `word.tm2`, `Loading.tm2` and the WALL archives are found through `Iso`'s release aliases.

## Not verified / not 1:1
- **Serve grades, CPU reaction draws and the aim narrowing were not compared frame by frame against the Japanese
  game.**
  - That needs the replay recorder rebuilt for the Japanese build: its globals, its smaller player and shot
    records, and the ball at other offsets. That is a recorder of its own, more than this session could do.
  - These rules come from the spec (a static decompile comparison) and our own tests: serve tables equal the
    rally table, no miss, no reaction draw, the 6.4/0.3 narrowing.
  - The disc was opened on the strength of those, the km/h oracle and the by-eye checks. Follow-up
    **M163a1a1** records the saved states frame by frame.
- **Court and umpire names on the Japanese disc are numbers** ("Court 2", "Umpire 5"), and the three descriptions
  are our own words. The disc's names are Japanese text inside its program, and the menus' letters can't draw
  Japanese. The front-end is ours to design (PLAN.md).
- **The character list's section is still titled "Hot Shots Tennis"** on the Japanese disc. It's cosmetic.
- **The speed readout follows our Units setting** (mph on a fresh settings file), not the Japanese game's fixed
  km/h. It's presentation.
- **Faces on the pause scoreboard are blank in `--shot` runs** on both discs, US included. They are rendered
  previews that shot runs don't draw, so this isn't a Japanese-disc gap.
- **Cut-away/instant-replay km/h and tornado km/h** still use the US formula, as M163a1 left them (presentation).

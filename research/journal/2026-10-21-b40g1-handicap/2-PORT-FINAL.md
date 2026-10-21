# B40g1 port: Set Handicap

Ported from `SPEC.md` with the decompile locked.

## What's in
- **Sim** (`hst-sim/src/player.rs`): `Body.handicap` (0/1/2, serde-default 0, so it is part of the netplay
  snapshot), `handicap_size` 100/80/70 (anything else 100), `Body::size()`. The human run step (`go`) and the walk-to
  estimate (`toward`) use it in place of the fixed 100.
- **Match** (`hst-play`): `play/handicap.rs`: the `--handicap a,b,c,d` resource (match slots), `apply` in `setup`, kept
  through each point's new body (`reset_positions`). The CPU run estimates (`approach`, `doubles_ai.rs` ×2) pass
  `p.body.size()`. `wear` hangs the weights: model by character/costume, the spec's placement rows, slot 0 on the
  right calf and slot 1 on the left. A mirrored (left-handed) body takes the other calf, its x axis turned and the
  mirrored offset, as the spec says. The weight is a child of the calf joint, so it shows exactly when the body does
  (cut-aways and replays too). It is lit as the body is through `character::WornGs` plus a fallback in
  `shade::own_materials`.
- **Menu** (`hst/src/play/main_menu/handicap.rs`): on the local Ready to play? screen, △ (keyboard L) on a player's
  own card or a computer's steps None → 1 weight → 2 weights. Each ready card's footer shows a pill ("No handicap" /
  "1 weight" / "2 weights") left of Ready!, and tapping it steps too. There is a legend entry, Handicap, and
  `Menu::args` adds `--handicap` in match order. `HST_MENU=confirm-handicap` shows it.
- **Online**: none. The lobby doesn't carry it yet, so the flag is never sent online, and both peers build every body
  at handicap 0. Follow-up: B40g3.

## Verification
- Oracle, `hst-oracle/tests/ai.rs` `run_frames_handicapped`: 20 000 run estimates, all at size 80 or 70 (about 10k
  each), bit-exact against the game's own function. `run_frames` (mixed sizes) still passes.
- Sim, `hst-sim/tests/player.rs` `b40g1_handicap_slows_the_run`: for handicaps 0/1/2, every running frame's velocity
  equals `run_velocity(dir, run_speed(…, 100/80/70))` bit for bit. Stamina and the run count go as usual, the
  handicapped bodies cover less ground, and the walk-to estimate's stop distance shrinks with the size.
- Menu/app, `main_menu::handicap::tests`: △ steps own and computer cards, not another person's. The flag goes out in
  match order (doubles: menu player 2 → slot 1). `setup` gives sizes 80/70 and spawns the weight parts (1 + 2
  weights).
- The original, on PCSX2: save slot 2 walked to Confirm Settings, then Set Handicap (Up raises a player's level,
  ✕ closes) at 1P x1, 2P x2, 4P x2, 3P x1. At the serve the players' size reads [80, 80, 70, 70] in match order, so
  the 100/80/70 mapping and the menu → match order are the port's. (Poking the settings byte before Start doesn't
  stick: the menu rewrites it.)
- Looks, by eye (`context/b40g1/`): the port's `--shot` of the same doubles (Ashley left-handed x1, Cody x1, Jun x2,
  JJ x2) frames almost like the original's capture (`orig/g1.png` vs `port_d.png`, crops `cmp.png`):
  - left-handed Ashley's single weight is on her right lower calf in both;
  - Jun's two sit on both lower calves at the same height.

  Close-ups in the port (a follow camera, not committed; `grid_s.png`): right-handed Ashley x1, Carol (lefty) x2, JJ x1
  (own model), Big Chief x2 (own model, 1.3×), Kent costume 0 x1 and costume 5 x2 (`weight_pc09_c`, over the hakama),
  all just above the ankle.
- Netplay (`Body` is in the snapshot): `HST_NETPLAY=1 tools/check.sh -p hst-play netplay`, 35 passed.

## Not verified / not 1:1
- JJ, Big Chief and Kent's costume model were only checked in the port, not side by side with the original. The
  original's capture shows JJ at the far baseline, too small to judge. Their rows and models are the spec's.
- The weights' materials are drawn two-sided. The rig's lighting sets the mirror flag on every draw of a left-handed
  rig, so one-sided faces would vanish on lefties. Looks only.
- Whether the weights cast a shadow in the original wasn't checked. In the port they are ordinary meshes under the
  rig and are drawn as the shadow pass treats them.
- Online matches play without handicaps (B40g3 carries them through the lobby and the handshake).
- The original's Set Handicap sub-screen (its own strip of per-player buttons) isn't copied. The front end is ours
  (PLAN.md), so it's a △ / pill on each card instead.

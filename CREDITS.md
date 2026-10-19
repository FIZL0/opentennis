# Credits and how OpenTennis is made

## The game

OpenTennis is a fan remaster of *Hot Shots Tennis* (PlayStation 2, 2007, Clap Hanz / Sony Computer Entertainment).
None of the game is in this repository or in our builds: no code, models, textures, sounds or text. The game reads
all of that from your own copy of the disc when it runs. Trademarks are their owners'; see the README's *Legal*.

## Built on

| Project | Licence | What we use |
|---|---|---|
| [Bevy](https://github.com/bevyengine/bevy) | MIT / Apache-2.0 | engine, ECS, rendering; patched copies of `bevy_render` and `bevy_pbr` in `third_party/` |
| [wgpu](https://github.com/gfx-rs/wgpu) | MIT / Apache-2.0 | graphics API; patched copy in `third_party/` |
| [winit](https://github.com/rust-windowing/winit) | Apache-2.0 | windows and input; patched copy in `third_party/` (Android pads) |
| [gilrs](https://gitlab.com/gilrs-project/gilrs) | MIT / Apache-2.0 | gamepads; patched `gilrs-core` in `third_party/` |
| [GGRS](https://github.com/gschup/ggrs) | MIT / Apache-2.0 | rollback netcode; patched copy in `third_party/` |
| [iroh](https://github.com/n0-computer/iroh) | MIT / Apache-2.0 | peer-to-peer connections for online play |
| [M PLUS Rounded 1c](https://github.com/coz-m/MPLUS_FONTS) | SIL OFL 1.1 (`crates/hst/assets/fonts/OFL.txt`) | menu font |
| [ps2-float](https://github.com/FIZL0/ps2-float), [ps2-oracle](https://github.com/FIZL0/ps2-oracle) | MIT / Apache-2.0 | our own: PS2 float arithmetic, and the interpreter the tests compare against |

Plus the crates in `Cargo.lock`, each under its own licence. The patched copies in `third_party/` keep their
upstream licence files.

## Tools

[PCSX2](https://pcsx2.net) runs the original game so we can record it and compare against it.
[Ghidra](https://ghidra-sre.org) reads the game program on our own machines. Neither is shipped.

## How it's made

- **Who wrote it.** Most of the code was written by AI agents (Claude, through Claude Code) working from a task list,
  with design, direction, testing and every release decision by the maintainer.
- **What it matches, and how we know.** Gameplay (ball physics, shots, AI, scoring, timing) aims to play exactly
  like the original. That is checked against the original itself: recordings of the real game running in PCSX2
  that our build must reproduce frame for frame, and a test harness that runs the game's own routines on your disc
  side by side with ours over thousands of inputs. Rendering, menus and presentation are our own design, made to look
  and feel right.
- **What stays local.** Research on the original (decompiler output, memory dumps, recordings, screenshots) stays on
  the maintainer's machine and is never committed. `research/` holds only our own scripts and notes.
- **Checks.** Every commit goes through `tools/audit.sh`: no addresses from the game program and no
  decompiler-generated names in the code or on the website, and no decompiled code pasted into the notes.
  Gameplay ports since 2026-10-19 are written in two sessions: one writes a behaviour spec in plain words, a second
  writes the code from that spec and the recordings without access to the decompiler output (PLAN.md,
  *Clean-room ports*).
- **What that doesn't prove.** Passing the audit only shows those patterns are absent. The two-session split
  is a working practice on one machine, not a sandbox, and ports from before it were written with the decompiler
  output open. None of this is a legal opinion.
- **One documented exception.** `crates/hst-data/src/exe.rs` reads tuning tables that only exist inside the game
  program from your own disc, at fixed offsets, after checking it is the US release (SCUS-97610). The tables
  themselves never leave your machine.

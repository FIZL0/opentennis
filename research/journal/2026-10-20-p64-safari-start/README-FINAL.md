# P64: Safari start-up and local Random picks

User (2026-10-20): the web build on Safari loads for a long time, apparently on the local random characters step.
No Safari device here: profiled in WebKitGTK's MiniBrowser (Safari's engine and OPFS on Linux) and the native build.

## What scales with installed mods at start

The page (`web/index.html` `start`) listed every installed mod one file after another: an OPFS round trip for each
directory entry and a `getFile()` for each file (also the lazy ones, only for their size), with the boot line showing
"Reading mod <id>…" through every mod. With the user's native mods folder (107 mods, 10 254 files, 8 573 after the
B227 `upscaled` cleanup) that's the one start-up step that grows with installs, and each step is a cross-process call
in Safari, much slower on a phone.

`tools/web_opfs_mods.mjs` (new) copies a native mods tree into WebKit's OPFS (manifests and pictures whole, others
1-byte stand-ins; `FULL=1` whole) and times the old loop against the page's current one (cut from index.html between
its `P64 listing` markers), checking both hand the game the same files:

| run (WebKitGTK, desktop) | old | new |
|---|---|---|
| 107 mods, first copy profile | 1340 / 1487 / 1702 ms | 356 / 562 / 790 ms |
| 107 mods, game's origin | 673 ms | 257 ms |
| 20 mods (full files) | 68 ms | 26 ms |

Same 8 573 files every run. Full game start with 107 mods in WebKit: mods read in 327 ms (old) / 295 ms (new), the
splash up ~3 s after load either way: desktop WebKit doesn't reproduce a long stall, so the phone's figure is unknown.

Change: all mods at once (8 at a time), and inside each its folders and files at once (16 at a time); the boot line
now reads "Reading mods… N of M". `hstModLoad` (a mod's models/textures/voices read in when the select or a match
wants it, e.g. a Random pick of a mod) reads its files 16 at a time too.

## Local Random picks at Start

A launch waits (≤ 3 s) for the picks' face close-ups (`previews::picks_drawn`, added 2026-10-19). But the baker's
queue loop only started keys some screen list named, and the picks on `Playing` weren't in it, so Start waited out
the cap and the drawing never came. A Random pick is the usual undrawn one: the select shows the ? card, and the last
player readying (often a COM on Random) goes straight to the confirm, which queued nothing locally.

Change (`previews.rs` `bake`): the players' picks are drawn on the confirm and after Start, ahead of the background
queue. Native check from the splash with Enter every 2 s, both players on Random, fresh card cache:

- old: 2P's pick (`d05-9`) never drawn; Start → "starting the match" 1.0 s later, HUD with the disc face
- new: 2P's pick (`d09-5`) drawn on the confirm 0.1 s after it was readied; Start → match start in 0.1 ms

WebKit with 20 real mods and a Random draw of a mod (`fore_pc02_emma`): read in at ready (26 ms), match started 61 ms
after Start (that mod ships its own picture, so no drawing was needed).

B589 (random-picked mods have no baked images) is likely the same queue gap; noted on it.

## Not verified / not 1:1

- Real Safari (iPhone/Mac) not run: no device here. WebKitGTK shares the engine and OPFS code but not Apple's
  storage process or a phone's speed, so how long the old start took on the user's Safari, and how much the parallel
  walk saves there, is unmeasured; desktop WebKit shows 2–3× on the listing.
- The old launch waited 1.0 s, not the 3 s cap, in the native run; not looked into (the wait is gone either way).
- Menus/front-end only: no gameplay, sim RNG or match timing touched.

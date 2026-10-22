# B643: iPhone web app went white and reloaded mid-match online

User report (2026-10-21): iPhone 14 Pro, web app, online match: the page flashed white, reloaded itself and dropped
out of the game. A white flash plus reload is WebKit killing the tab's WebContent process (memory), or the game's own
reload after a lost GPU device (`web/index.html`, B121, which iOS also does under memory pressure). Either way it is
memory, and either way the page comes back on the menu, so two questions: what eats memory in an online match, and
does M126's Rejoin bring the player back after the reload.

## How it was measured

No iPhone or Safari Web Inspector here. Instead: the browser build (`web/pkg`, WebGL2, wasm-opt'd, the same build id
as the host via `HST_COMMIT`) in headless Chromium (SwiftShader, 1280×720) as an online guest
(`?code=…&autoplay&log`; `?autoplay` is new: every seat a bot, as `HST_AUTOPLAY` natively, so a long online match plays
with nobody at the page) against a native `hst net host --bots --games 6`. Every 10 s: the game's own wasm heap line
(`hst: wasm memory … (live, peak)`), `performance.memory`, and the RSS of Chromium's renderer and GPU processes.
Control: the same page offline (`?play&pads=-,-,-,-`). Scripts and logs in `context/B643/` (`online.mjs`, `run3`,
`off1`, `run5`).

## What was found

1. **The waiting notice's 1600 quads were spawned and despawned over and over.** The online notice ("Waiting for
   1P...", `main_menu/pause.rs`) keeps a pool of 1600 UI quads only while its text is non-empty (P39). In an online
   match where one side is slow (the guest here ran ~7 fps on SwiftShader; a phone that can't hold 60 is the same),
   the lockstep holds for more than `STALL` (0.25 s) every few ticks and catches up straight after, so the text
   flips empty ↔ "Waiting for …" every few frames — and each flip spawned or despawned all 1600 entities. run3's
   stall lines show `entities 4018 (+1600)` / `2418 (-1600)` alternating for the whole match (1600+ flips logged
   just on the frames slow enough to be logged); offline (off1) the count sat flat at 1412–1416. On the web every
   spawn is UI nodes, render-world entities and their buffers, and the wgpu web backend leaves freed GPU objects to
   the JS garbage collector (B468): a steady stream of that is exactly what pushes an iPhone tab over its budget.
   In run3 the renderer process grew ~11 MB/min through the online match (1136 → 1221 MB), the offline control did not.
   Fix: the pool stays (hidden, which `notice_draw` already does for unused quads) for 10 s after the notice goes
   down (`NOTICE_LINGER`), so a flickering notice keeps one set of quads. The flashing itself (showing after only
   0.25 s) is B642's.
2. **Rejoin only worked in the first 10 minutes of a match.** The M126 record is written once, when the match
   starts, and counts as fresh for `FRESH` = 600 s. A reload 10 minutes or more into a match (the user's case: long
   online match) found it stale and came back to the menu. Now `rejoin::keep` rewrites the record with the time now
   every 60 s while the match goes on (from `recover::tick`, only if this machine has one and it hasn't been
   forgotten); a test covers a record kept across 3× `FRESH`.
3. **Rejoin by short code only worked in the first ~2.5 minutes.** Found by actually reloading the guest page
   mid-match (run6, 650 s in): Rejoin came up (fix 2) but the lookup said "The lobby PKHN stopped answering 13
   minutes ago": the host stops republishing its short code once the match starts (the relay loop never lists it),
   so the record's short code goes stale after `STALE` (150 s). Now the guest remembers the ticket its short code
   led to (`rejoin::resolved`, set by the join's lookup) and the record keeps it as an optional fourth field; a
   rejoin taken from the record dials that ticket once (`rejoin::dial`) instead of looking the code up. Older records
   (three fields) still read. Test `a_rejoin_dials_the_ticket_its_code_led_to`.
4. Measured, not changed:
   - wasm heap: 423 MB on the menus, 559 MB after the online match loaded (live peak 429 MB during the load,
     ~300–313 MB live while playing), 631 MB after going back to the menu. Wasm memory never shrinks, so the
     load's peak is what the tab keeps; offline the match sat at 473 MB.
   - Textures held in the match: ~246 MB, the biggest the menus' letter sheet, 4096×2304 RGBA with mips (50 MB),
     rebuilt on each menu↔match switch. Follow-up B644 (smaller sheet on touch/low-memory browsers, needs A/B shots).
   - JS heap tiny (13–23 MB); asset counts flat over the match.

## Verification

- `tools/check.sh -p hst-base rejoin` (incl. the new `a_kept_record_outlives_the_match_start`),
  `tools/check.sh -p hst-play recover`, `HST_NETPLAY=1 tools/check.sh -p hst-play netplay` (36) pass.
- `tools/check.sh -p hst-base net::` (47) pass.
- run6 (the notice fix, same setup, service worker bypassed so the page really ran the new build; run5 had been
  served the cached old one and still churned 234 times in 210 s): the count went +1600 once when the notice first
  showed and stayed at 4024; the only other ±1600 are ~10 s apart (the linger letting go after a real quiet spell).
  Reloaded 650 s after the start: `online: rejoining the match left at PKHN` (stale before fix 2), then fix 3's
  failure above.
- run7 (all three fixes): page reloaded 240 s into the match (short code already stale), Rejoin came up, one press,
  the host logged `Player (…) rejoins for …`, the match went on with both seats human and the guest back on court
  (screenshot in `context/B643/run7/rejoined.png`). The rejoined guest then reported a desync once and resynced
  from the host's state ("in sync from tick 100"): existing behaviour after a restart, not chased here.
- Renderer RSS still climbs ~15 MB/min in run6 (1137 → 1306 MB over 600 s), also on the menu after the reload, while
  the offline control rose ~6 MB/min and fell back (GC) at ~400 s. Not explained; the wasm heap was flat
  (560 MB, ~320 MB live) through the match, so it's browser-side (GPU objects / command buffers).

## Not verified / not 1:1

- Not run on an iPhone or in Safari: no device or Web Inspector here. All numbers are headless Chromium with
  SwiftShader WebGL2 on Linux; WebKit's per-tab limit and how it counts GPU memory differ, so whether these cuts
  alone keep a 14 Pro under its budget for a whole online match is not proven.
- Whether the user's crash was the WebContent kill or the game's GPU-lost reload can't be told from here; both are
  memory, and Rejoin covers both.
- The menus' letter sheet (50 MB) and the wasm heap's load peak (~440 MB live while an online match loads, against
  ~300 playing) are left for B644 / B645; not cut here.
- The renderer-process growth online (above) is unexplained; a Safari timeline on a real iPhone would show whether
  WebKit grows the same way.
- `?autoplay` (every seat a bot) is a test switch for runs like these; both ends need it or they go out of sync.
- Online/rejoin is ours (no original to match); nothing here touches the sim.

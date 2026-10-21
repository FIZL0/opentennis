# B607 — menu → match transition out of order

## What happens (`crates/hst-play/src/mode.rs`)
A switch first asks the running mode for its last frame (a window screenshot, `leave`), puts that frame up as a
full-window image over everything (`Snapshot`), then puts the loading screen up under it (`show_loading`) and zooms
and fades the frame out over it (`zoom`); the new mode is built once the fade is done.

## Why it went wrong
- `leave` waited **10 frames** at most for the screenshot. A screenshot read-back of a big window on a fast display
  (or a busy GPU) can take longer: the loading screen went up without it, and when the capture landed a few
  frames later the observer still spawned the snapshot over the loading screen. The player select came back on top of
  the loading screen, then zoomed and faded: the order the user saw.
- The fade clock started when the snapshot was spawned, before the loading screen was under it, so its first
  frames already faded towards the still-running menu, and the loading screen could go up before the
  snapshot's texture had been drawn once.

## Fix
- The wait for the capture is 10 frames **or** 0.5 s real time, whichever is longer.
- A capture that lands after the loading screen is up is dropped (never shown over the loading screen).
- The loading screen goes up only after the snapshot has been drawn twice (its texture on screen, covering the menu).
- The zoom/fade clock runs only while the loading screen is up under the snapshot.

## Verified
Local match, 1280×720 window, `HST_MENU=confirm` + `HST_KEYS` to start, with a temporary per-frame screenshot system
(not committed). Before: the snapshot spawned and faded from frame 0 while the loading cover went up the next
frame. After (frames logged per frame): snapshot spawned → drawn 2 frames over the menu (no change on screen) →
loading screen goes up under it → 0.5 s zoom/fade into the loading screen → loading screen → match (shots after the
loading screen came down show the court, no player select). `tools/check.sh -p hst-play mode::` passes.

## Not verified / not 1:1
- The user's exact "loading first, player select back" sequence wasn't reproduced on this machine: here the capture
  came back in 3 frames. The fix covers the late-capture path by construction (dropped when late, longer wait), but
  a late capture wasn't forced in a run.
- Online (host + join) wasn't run: it needs two networked instances. The online start uses the same `mode.rs`
  switch (`Next` with the netplay-wrapped match mode), so the same order applies; not checked live.
- Front-end presentation, so no comparison with the original (menus/transitions only have to feel right).

# B706a — GitHub #4 follow-up (Adreno 730 players)

## State (2026-11-14)
- #4 is open. Last activity: the owner's comment (17:28 UTC) asking the reporter to try the next release, or the
  `debug.txt` switches (`gpu_preprocess = on`, `uniform_skins = off`) and send a new `hst-log.txt`.
- No reply from the reporter yet, so there is nothing to confirm and no new log to read.
- The B706 fix (merge 7af1d9a3) ships in **v1.6.3-beta.1** (pre-release, 18:16 UTC, has `opentennis-android.apk`).
  The latest stable is still 1.6.2, which does not have it. The comment says "once the next version is out"; the
  reporter may be waiting for a stable release, or may not know about the beta.

## Next step (needs the human / the reporter)
- Optionally tell the reporter on #4 that v1.6.3-beta.1 has the change (I did not post: publishing on the user's
  behalf in an unattended run), or wait for the stable 1.6.3.
- When they reply: players draw → close #4. Still broken → read their log's first lines (`skins and morph weights in
  uniform buffers` present or not, and which `debug.txt` switch helped) and open the next task from that.

## Not verified / not 1:1
- Everything: the fix is untested on an Adreno 730 until the reporter answers. No code changed in this task.

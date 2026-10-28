# M152g: a native app with no disc joins an adhoc host

## What changed
- **No-disc page → "Join a game nearby"** (desktop: new `adhoc::page::no_disc`, which also has "Choose disc image…";
  Android/iOS: a button on their existing no-disc pages). The overlay (`adhoc/page.rs`) lists the hosts it finds and has an
  address row with on-screen keys (digits, `.`, `:`, ⌫) for typing `192.168.x.y[:port]`. It works with pad, keys, mouse
  and touch, and is sized in vmin so it fits any aspect ratio.
- **Finding hosts**: an adhoc host (`HST_ADHOC` / the host's adhoc switch) answers UDP broadcasts `opentennis-find 1` on
  port 47152 with `{"game":"opentennis","port":…,"name":…}`. The guest re-broadcasts every 2 s to 255.255.255.255, to
  each interface's broadcast address and to loopback, and drops hosts it hasn't heard from in 7 s.
- **Download** (`adhoc/guest.rs`): it reads `/adhoc/info.json` and checks the build (a mismatch is refused unless one
  side is a dev build). It then streams `/adhoc/disc`, retrying while the host answers 503 (still packing), checks the
  pack hash, and waits until the host's lobby is up.
- **Kept in memory only**: a Bevy window can open only once per process, so the guest restarts itself. Desktop
  execs itself (the AppImage path if set); Android uses `HstActivity.restart`; iOS asks the player to reopen. The pack
  waits in `<data>/cache/adhoc-guest/` during that restart. The first thing the next start does (`disc::resolve` /
  `android_main`) is `guest::resume`: it reads the pack into RAM, **deletes the folder**, and mounts the pack at a
  virtual path through `iso::mount`. Nothing reusable stays on disk. An error or a cancel also deletes the folder.
- **Joining**: `online.rs::linked()` auto-joins `rtch<host:port>` (`guest::code()`) just like `HST_CODE`, so it uses
  the same WebRTC join as a browser guest.
- **Mods**: `guest::mods` puts the host's listed mods into `modfs` like the browser guest (M152c): small files and
  texture packs at once, everything else lazily on first use through the new `modfs::fetch_with` hook. Native code
  before this change never had lazy entries.
- `tools/native_guest.sh DISC OUT [secs]`: an end-to-end run with a host and a disc-less guest on this machine.

## Verified
- `tools/check.sh -p hst-base adhoc` and `modfs`: these include `a_guest_gets_the_disc_and_keeps_it_in_memory`, which uses
  its own server and its own UDP port. It finds the host, downloads the pack, resumes it and checks that the pack hash matches
  and the folder is gone.
- E2E (`tools/native_guest.sh … context/m152g/run4 60`): the guest starts with an empty data folder and no disc. It
  finds the host, gets the disc, restarts, joins the host's lobby (screenshot: the guest's lobby, "1 of 2 ready"),
  presses Ready, and the host starts the match. The rollback hashes match on both ends (tick 0 `cd21f7460d38ee59`,
  tick 1800 `3df3ce306392f034`), and points are played. Afterwards the guest's data folder holds only settings,
  controls, preview PNGs and the pipeline list. No disc or pack is left.
- Build checks pass with no warnings: native, Android aarch64 (`--example hst --features android,ship`), iOS
  (`tools/ios-build.sh check -p hst`) and Windows gnu (`-p hst-base`).

## Not verified / not 1:1
- Not 1:1: this is a new feature with no counterpart in the original.
- Not run on a real Android phone, iOS device or ROCKNIX handheld: those only have compile checks. ROCKNIX goes through
  the desktop path (exec self). This was not run there.
- iOS can't restart itself: the player is told to reopen the app. The downloaded pack waits in the cache until then
  and is deleted on that next start.
- Between the download and the restart, the pack is on disk in `cache/adhoc-guest` for about a second (or until iOS
  is reopened). A crash in that window leaves it there until the next start, which loads it and deletes it.
- Broadcast discovery can be blocked by firewalls or by AP client isolation. Typing the address still works then.
  Only one host per machine can answer on port 47152; the test uses its own port.
- Mods: the lazy download of large mod files was covered only by the unit test of the path split. The E2E host had no mods.
- `HST_JOIN` (auto-pick a host) is for dev and tests only; ship builds strip `HST_*`.
- The guest's own settings (controls, graphics) persist in its data folder like any install. This is by design: they are not game data.

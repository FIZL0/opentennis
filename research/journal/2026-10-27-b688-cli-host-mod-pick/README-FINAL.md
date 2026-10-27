# B688: CLI host stalls on a guest's mod pick

## Cause

With a mod in the line-up, the host's start (after the countdown) waits until every peer, the host included,
has said its looks for that line-up are settled (`Cmd::Ready(ver)`). The menu sends that once each mod's looks
are found, fetched or turned down. `net::cli` never sent it, so a CLI host (or CLI guest) with a mod in the
room showed the countdown and then never started.

Second gap, found along the way: `hst net … --play` built its match from `netplay::flags` only. No `MatchMod`
went in, so a mod seat would have played its donor's disc character on the host and the mod on the guest.

## Fix

- `net::cli`: on every new line-up (`Room.ver`) it sends `Cmd::Ready(ver)` at once. There is nothing to fetch
  there: the match takes the looks from the mods folder or the donor stands in.
- `main.rs` `cli_mods`: the `--play` match gets the room's mods in their match slots, in the same order as the
  menu's online start. Each one uses the wire's gameplay and its looks from `mods_online::find` (the mods folder
  `modfs::root()`, the same one adhoc serves, or a copy received before). If neither has them, the donor stands
  in. They are preloaded like the menu's.
- Menu host: checked by reading the code. `settle_looks` finds a guest's mod that the host has installed by id and
  looks hash (adhoc serves the host's own files, so the hash is the same) and sends Ready. If the host doesn't
  have it, it fetches it, asks or uses the stand-in, as for any online mod. No change was needed.
- `tools/web_guest.sh`: its comment no longer says mods stall the CLI host.

## Runs (`context/b688/`)

`HST_COMMIT=b688test` on both the release host and a rebuilt `web/pkg`. Run1 was refused because `web/pkg` was
from another commit. The host ran in OUT with `mods` → the user's mods folder.

- run2: the guest drew a disc character. Started, hashes agree to tick 1800.
- run3: the guest drew mod Gloria. Countdown → Start at 37 s, tick 0 hashes agree, the match plays on the guest
  (screenshot).
- run6 (100 s): the guest drew mod Shu. Started; host and guest hashes agree at ticks 0/1800/3600/5400.

## Not verified / not 1:1

- **Menu host with a guest's mod pick**: not run live. I only read the code (see above); the existing
  `online.rs` tests cover its Ready-on-settled.
- **A CLI host with a mod it doesn't have**: not run. The donor stands in on the host and the gameplay is the
  same (B556).
- Not 1:1: online play has no counterpart in the original.

# B612 — refuse discs whose contents aren't a known retail copy

- `hst_data::iso::KNOWN_PACKS`: the pack hashes of known-good copies, starting with our US 1.00 copy
  (`d89b5d230259faef`, from a fresh import of the fixture ISO; the same as the user's Oct 12 pack header, `packed`
  hasn't changed since M13).
- `write_pack_to` checks the finished hash against the list before writing it into the header; a miss is an
  `InvalidData` error from `iso::unknown_pack`: "unrecognised disc contents (bad rip or different version), pack hash
  …; if your disc is a good copy, report this hash so it can be added". `write_pack` deletes its `.part` then.
- Being in `write_pack_to`, it covers every import: desktop (`disc.rs`: "Couldn't import <iso>: <error>"), Android
  (InvalidData → "That's not the disc this game needs (<error>) …") and the browser (the error string goes to the page).
- B604's ISO directory checks stay (the walk must parse before anything can be hashed); the SCUS-97610 disc-ID check
  stays too and still runs first (a wrong disc fails fast, before the 170 MB copy).
- Tests (`hst-data --test pack`): our copy's hash is in the list; an ISO read through a reader that flips one byte in
  the middle of a packed file is refused with the message and leaves no pack or `.part`. Checked the test isn't
  vacuous by breaking the listed hash once (the good-copy test then failed with the message and hash).
- Note: changing `packed` (what goes in a pack) changes every hash; `KNOWN_PACKS`' doc says to re-import a known copy
  and update the list.

## Not verified / not 1:1

- Only one known-good copy (ours). Other good US 1.00 rips should give the same hash (the hash covers file contents
  and the pack's own index, not the image layout), but no second rip was available to confirm it.
- The browser page's display of the new error wasn't run (it shows the worker's error string as before).

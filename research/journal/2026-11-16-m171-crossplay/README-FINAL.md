# M171: every release hosts and joins every other (crossplay)

## What landed

- **`hst-data::digest::survey`**: the gameplay digest (unchanged value) plus a `Manifest`, one hash per source
  (program tables, each TRAJ/PCDATA/PCCG0/CMN/EFFCT/INPANE/PCANI/costume/court archive), so a host can see which
  parts of a guest's disc play differently.
- **`hst-data::crossplay`**:
  - `patch_for(host disc, guest manifest)` builds a `Patch`. It holds the whole program files (SYSTEM.CNF, GAME.BIN,
    MENU.BIN) when the tables differ, plus every counted entry of each archive that differs.
  - It is encoded with the `HSTXPLY1` magic.
  - `install`/`verify`/`remove` keep a per-disc-path overlay in memory, never written to disk.
  - `Iso::open(path)` reads through the overlay. Each covered archive is rebuilt from the guest's own uncounted
    entries plus the host's counted ones.
  - Voices, text, textures and every file outside the patch stay the guest's.
  - `verify` re-digests through the overlay and refuses unless it equals the host's digest.
  - `Iso::open_own` reads the disc without the overlay. The handshake uses it for the disc's own digest and rules.
- **Handshake (`hst-base::handshake`)**:
  - The `Hello` now carries the manifest. A peer with a different digest that sent one is let in (`needs_data`); a
    peer that sent none is refused, as before.
  - New functions: `rules()`/`rules_of`/`rules_name` (US / Japanese / European, from SYSTEM.CNF), `rules_differ`
    (the pop-up's lines), `patch_for`/`take_patch`/`end_crossplay`.
- **`net.rs`**:
  - `Room` carries the host's `rules` and `sim`; `Listing` carries `rules`.
  - The host sends the patch on a pack stream keyed by its digest (`crossplay_send`, built off the network thread).
  - The guest (`Crossplay` in `join_as`) sends `Fetching(Some(0))` on its first line-up from another digest and
    reports progress. It verifies the patch off-thread, then sends `Fetching(None)` and a status line. On failure it
    gets `Failed` and the connection closes.
  - Leaving the lobby (`join` returning) removes the overlay.
  - A crossplay guest that rejoins mid-match is refused with a reason (follow-up M176).
- **Menu (`main_menu/rules_note.rs`)**:
  - The lobby list rows and the card's lobbies show "Singles, Japanese rules" when a lobby's rules aren't this disc's.
  - A guest joining such a lobby gets a pop-up: "This lobby plays the Japanese rules", the differences in player
    terms, and OK (✕/○ or a click off the box closes it).

## Proof

`play::netplay::tests::matches::a_japanese_disc_plays_a_us_hosts_match` and `a_us_disc_plays_a_japanese_hosts_match`
(`HST_NETPLAY=1 tools/check.sh -p hst-play netplay::tests::matches::a_`, about 10 s together). Each one:

1. **Control:** the guest plays on its own disc in the host's match. They go out of sync (desync report saved).
2. **Patch:** builds the host's data for the guest's manifest, round-trips it through the wire encoding, and installs
   and verifies it on a temp symlink of the guest's disc.
3. **Lockstep:** in sync over 894 ticks (no desync note), both peers on the host's build (`[Us, Us]` / `[Japan, Japan]`).
4. **Rollback (ggrs, 3-tick wire):** in step over 795 ticks, no desync.

Patch sizes: US host → JP guest 12.6 MB, JP host → US guest 12.7 MB.

Other tests:
- `hst-data crossplay`: a patch round-trips; a patched JP disc digests as the US disc and the other way round; voices
  stay the guest's.
- `hst-base handshake`: `rules_and_how_they_differ` and the extended digest test.
- `hst rules_note`.

## Not verified / not 1:1

- **No real two-machine network run.** The host sending and the guest verifying over QUIC (`crossplay_send`,
  `Crossplay`) is covered only by building and by the in-process tests of each half. The handshake's disc globals are
  one per process, so a two-disc net test can't run in one test binary.
- **No PCSX2 check of the pop-up's lines.** They are written from `hst_sim::build::Build`'s rule list and M163a's
  findings (serve timing, strong-serve miss, special serves/counters, lob/slice aim width, smash narrowing, CPU lob
  reaction, red marker on serves, per-character TRAJ differences). "Smashes narrow only deep in the court" vs "from
  mid-court" paraphrases the spec's smash-narrowing difference.
- **Patch size (~12.5 MB).** It holds the whole program files and every counted entry of each differing archive. A
  diff could send less; the size is within the pack cap (128 MB).
- **Browser LAN (rtc) peers** get no patch: `send_pack` is QUIC only, as for mods. In the browser the host builds the
  patch on the main thread, which stalls the page briefly.
- **European disc:** not playable yet (M174). `rules_differ` lists nothing for it, and the list shows "European rules"
  only once such a host exists.
- **Lobby list:** shows the rules only when they differ from this disc's, not on every lobby. Same-release lobbies
  look and play exactly as before.
- **Rejoining a crossplay match** is refused for now (M176).
- The umpire's call timing and other program-side timings come from the host's GAME.BIN via the overlay. Only the
  digest and the match-state hashes check them, not a recording.

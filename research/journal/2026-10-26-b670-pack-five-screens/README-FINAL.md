# B670: texture packs miss five screens

## Finding, per screen
| Screen | Game side | Pack side (hst_hd v1.2) |
|---|---|---|
| Loading hint (`l_txtNN.tm2`) | `loading.rs` added it with no replacement lookup | key missing |
| Loading court picture (`l_bgNN.bmp`) | no lookup, and a BMP had no texture key at all | missing (could not exist) |
| Loading title card (`loading.tm2`) | a rebuilt menu sheet (`menu_art` SHEETS): never looked up | key missing |
| Intro court plate (`courtpreview_00`, `_nameNN`) | already through `panel::image` → `add_tim2` | keys missing |
| Start popup (`inpane_start00–02`, `i_gamestart_00` model) | 00 and the model already looked up; 01/02 rebuilt sheets, never looked up | keys missing |

So both sides were short: the pack had none of the 66 textures, and four of the paths never asked.

## Game changes
- `hst_data::bmp`: the 8-bit BMP decoder (moved out of loading.rs) plus a key for a BMP (an xxh3 of the file, tagged
  "BMP"; BMPs never reach the GS so they have no GS key). `texhash::disc_textures` now lists BMPs, so
  `--dump-textures`, `texkeys` and the store's pack upload check all know them (the new lines: the court
  pictures, a few reflect/title/sky/challenge-wall BMPs).
- `textures::has` (a pack or texture-replacements has this key/names), `textures::replacing` (the setting is on,
  browser included; `on()` is always false there), `textures::add_bmp`.
- `loading.rs`: the hint goes through `add_tim2`, the court picture through `add_bmp`.
- `menu_art::sheet_image`: a rebuilt sheet the replacements have a picture for is not rebuilt; it goes through
  `panel::image` (`add_tim2`) like any other disc sheet. No pack has any other SHEETS entry today, so only
  loading / inpane_start01 / inpane_start02 change, and only with a pack that covers them.
- The P57 defer test used `textures::on()` (false in the browser), so during a mode start in the browser, the
  non-rebuilt sheets skipped the pack. Now it uses `replacing()`. Desktop behaves the same as before.
- Replacements off: every path ends in `images.add` of the same image as before (unchanged).

## Pack
66 native-size sources (alpha as the game draws them: `decode_alpha8` for the 2D sheets, as the pack's own
`info` file shows; MTL as dumped; BMPs opaque) went through `tools/chainner/upscale.chn` with 4xHDcube4
(`~/.local/bin/chainner-app/Models/HDcube/4xHDcube4.pth`, the model the chain was made for), via `upscale_mods.py`'s
`upscale()`. All came out at exactly 4×, with a mean alpha/colour difference of ≤2.1/2.8 levels after box-downscaling them back. Court pictures
were saved as RGB. Copied into `~/.local/share/hst/mods/hst_hd/textures/` under their dump paths (4309 → 4375
files) and `mod.json` was bumped 1.2 → 1.3. Not uploaded (M150).

## Verification
A/B `--shot`s in `context/b670/` (data folder `context/b670/disc`: ISO and only hst_hd, `upscaled_textures = on`;
`env -u HST_BG`, which otherwise forces replacements off):
- `run2/` loading screen (temporary, uncommitted `HST_B670` startup hook drawing `loading::cover` for court 6, as
  B606 did): with the pack, the logo, hint text, "Now Loading..." and the court picture are the upscale. With
  `--no-upscale`, it's the disc art and our rebuilt "Now Loading..." label, as before.
- `run4/` `--play --court 6 HST_INTRO=1`: the court plate's name and the "Start the Match!" banner/pill are
  upscaled in A and the disc's in B.
- `cargo test`: hst-data `bmp` (new) and `texhash`.

## Not verified / not 1:1
- The loading screen was shot through a temporary hook, not a real menu → match load (the cover is only up briefly).
- The pack's v1.2 upscaler settings aren't recorded anywhere. I used the repo's chain with 4xHDcube4. If v1.2 used
  4xHDcube4Plus-B, the new files may look slightly different from the rest of the pack.
- Only court 6's picture and hint 0 were looked at on screen; the other 10 pictures / 37 hints were only checked
  numerically (size, difference from the disc).
- Browser build not run (the `replacing()` defer change only affects it).
- Presentation/front end: compared by eye, no proof needed (PLAN).
- Also fixed two clippy errors already on main in `crates/hst-sim/tests/cutaway.rs` (from B665) that blocked the gate.

# M162 — macOS build for Apple Silicon (blocked on Xcode.xip)

**Blocked:** there's no `Xcode*.xip` in `~/Downloads` and no `MacOSX.sdk` anywhere on the machine (the earlier iOS
extraction kept only `~/.local/share/hst-ios/iPhoneOS.sdk`). The task says to block in that case: plan/HUMAN.md has the
download steps.

## What's in this round (inert until the SDK is there)

- `tools/ios-sdk.py` now pulls **both** `iPhoneOS.sdk` and `MacOSX.sdk` in one pass over the .xip, into
  `~/.local/share/hst-ios/`. An SDK that's already there is kept, so running it again on the new Xcode only adds
  `MacOSX.sdk` and leaves the working iPhone SDK alone. The `.tbd` fix (`arm64e.x1` → `arm64`) covers macOS target
  lists too, in case Xcode 27's macOS stubs have the same new arch. Its self-tests (asserts at import) pass. The
  "both already there" path was tried; the extraction itself can't be tried without a .xip.
- `tools/macos-build.sh`: the iOS one's twin for `aarch64-apple-darwin` (clang + rustup's ld64.lld, `-isysroot`
  MacOSX.sdk, deployment target 11.0, `HST_MACOS_SDK` overrides the path).
- `tools/macos-app.sh`: release build with `--features ship`, stripped, `OpenTennis.app/Contents/{MacOS/opentennis,
  Info.plist, Resources/AppIcon.icns}` (icns written from web/icon-512.png as PNG entries at 128/256/512/1024),
  ad-hoc signed with `rcodesign sign` (no certificate → ad-hoc), zipped with `zip -y` (keeps symlinks) as
  `opentennis-macos-arm64.zip`. Same bundle id as iOS (`io.github.fizl0.hst`).
- `tools/build.sh macos` → `dist/macos/opentennis-macos-arm64.zip`. **Not in `all` yet**: release.sh runs `all`, and a
  release mustn't fail on a missing SDK or ship a Mac app nobody has started.
- `rustup target add aarch64-apple-darwin` done on this machine (`macos-build.sh` checks for it).

The game side needs no change, as far as reading the code shows: `disc.rs` already puts files in
`~/Library/Application Support/hst` on macOS and the first start's ISO dialog is `rfd` (native AppKit on macOS);
`render_mode::NO_SOFTWARE` already hides Software on macOS; `pipelines::IN_PLACE` covers macOS; the mod store opens
links with `open`.

## Cross-check without the SDK

`cargo check --target aarch64-apple-darwin -p hst --features ship` passes with no errors and no warnings, so every
crate and every `cfg(target_os = "macos")` path compiles for the Mac. The C build scripts (blake3…) need system headers,
so for this compile-only check `CFLAGS_aarch64_apple_darwin="-target arm64-apple-macos11.0 -isysroot <iPhoneOS.sdk>
-Wno-incompatible-sysroot"` borrowed the iPhone SDK's libc headers. That's fine for `check`, but it can't link: linking
needs MacOSX.sdk's libSystem/AppKit/Metal stubs.

## Next round (after the user downloads Xcode and unblocks)

1. `tools/ios-sdk.py ~/Downloads/Xcode*.xip` → `MacOSX.sdk`; `cargo install apple-codesign` (rcodesign isn't
   installed yet).
2. `tools/build.sh macos`; fix whatever the link brings up (frameworks: Metal, AppKit, CoreAudio, IOKit for gilrs…).
   Check the bundle: `llvm-objdump --macho --private-headers` (LC_BUILD_VERSION macos 11.0, arm64),
   `rcodesign verify`.
3. Then the publishing parts, held back this round so nothing points at a file that doesn't exist yet: add `macos` to
   build.sh's `all`; release.sh copies `dist/macos/opentennis-macos-arm64.zip` into `dist/release`, checks it (zip
   has `OpenTennis.app/Contents/MacOS/opentennis`, signature present), uploads it, and adds a download-table row
   ("**Mac** (Apple Silicon, macOS 11+) | opentennis-macos-arm64.zip | unzip, drag to Applications; first time:
   right-click → Open, or on macOS 15+ System Settings → Privacy & Security → Open Anyway (not notarized); it asks
   for your ISO"); a macOS tile in web/index.html's `#download` (Apple logo, "Apple Silicon"); the README download
   section; the RELEASE_NOTES.md line. Note macOS 15 dropped the right-click → Open bypass for unnotarized apps, so
   the text has to give "Open Anyway" (or `xattr -dr com.apple.quarantine OpenTennis.app`) first.
4. Does `update.rs`'s self-updater know a Mac asset? It would need one (or to send Mac players to the release page).
   Read it when this lands.

## Not verified / not 1:1

- Nothing was built or run: no MacOSX.sdk (no Xcode.xip) to link against, no Mac to start the app on. The SDK
  extraction, the link, the icns, the signature and Gatekeeper's behaviour on the zip are all untested.
- Not a gameplay port: no original to compare against.

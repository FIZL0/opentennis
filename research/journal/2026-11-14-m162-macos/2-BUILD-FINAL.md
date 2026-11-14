# M162 round 2 — the Mac app builds, and releases carry it

The SDK arrived as `~/.local/share/hst-ios/MacOSX.sdk` (macOS 15.5, copied from the user's Mac), the default path
`tools/macos-build.sh` already reads, so the .xip extraction wasn't needed.

## What happened

- `cargo install apple-codesign --locked` → `~/.cargo/bin/rcodesign` (0.29.0). That folder isn't on the runner's PATH,
  so `tools/macos-app.sh` appends it.
- `tools/macos-build.sh build --release -p hst --features ship` linked first time, with no source changes (3.5 min).
  Mach-O arm64 executable, `LC_BUILD_VERSION` platform macos, minos 11.0, sdk 15.5. Dylibs: only system ones (AppKit,
  Metal, QuartzCore, CoreAudio, AudioToolbox, IOKit, Security, SystemConfiguration, Carbon, ColorSync, CoreVideo,
  libSystem, libobjc, libiconv). No Vulkan/MoltenVK: wgpu draws through Metal.
- `tools/build.sh macos` → `dist/macos/opentennis-macos-arm64.zip` (40 MB; the stripped binary is 117 MB).
  The zip holds `OpenTennis.app/Contents/{MacOS/opentennis, Info.plist, Resources/AppIcon.icns, _CodeSignature/CodeResources}`.
  `rcodesign print-signature-info`: flags ADHOC, identifier `io.github.fizl0.hst`, Info.plist and Resources slots
  bound. (`rcodesign verify` prints "CMS error": an ad-hoc signature has no CMS blob, and rcodesign itself warns that
  verify is unreliable.) Info.plist parses (plistlib): version 1.6.3, build 4733, LSMinimumSystemVersion 11.0.

## Publishing (held back last round, done now)

- `tools/build.sh all` builds the Mac zip too (release.sh runs `all`).
- `tools/release.sh`: requires `tools/macos-app.sh` to build with `ship` (B492 check); refuses a zip without a
  signed `OpenTennis.app` (binary + `_CodeSignature/CodeResources`); copies it into `dist/release` (so the existing
  globs put it into SHA256SUMS and the upload); download-table row: Apple Silicon, macOS 11+, unzip → Applications,
  right-click → Open, or on macOS 15+ open it once then System Settings → Privacy & Security → Open Anyway; asks for the ISO.
- web/index.html `#download`: a **Mac / Apple Silicon** tile (Apple logo) beside Windows/Linux, before Android; the banner's
  "your download" pill picks it for a `Macintosh` user agent (iPads, which also say Macintosh, are caught by the
  touch check first); macOS added to the JSON-LD platforms and the FAQ answer.
- README download line + the not-notarized instructions; RELEASE_NOTES.md line.
- Self-updater (`update.rs`): no Mac asset on purpose. `ASSET` stays `None` on macOS, so the update notice opens the
  release page instead of downloading. Swapping the binary inside the bundle would break the ad-hoc signature and
  Apple Silicon would refuse to start it. If a Mac self-update is wanted later, it has to replace the whole .app.

## Not verified / not 1:1

- **Never started on a Mac**: there's no Mac here. The link, the bundle layout, the signature and the plist are checked
  on Linux. Not checked: that it starts, that Metal renders correctly, the first-start ISO dialog (rfd/AppKit), pads
  (gilrs/IOKit), audio, and how Gatekeeper treats the downloaded (quarantined) zip on macOS 11–15. The user should try
  `opentennis-macos-arm64.zip` on their Mac before the next release ships it.
- ColorSync.framework is linked (pulled in by a dependency). If it's missing on the oldest macOS 11 releases, the
  minimum would have to go up. Unchecked.
- Not a gameplay port: nothing to compare against the original.

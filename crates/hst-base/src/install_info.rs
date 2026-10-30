//! M159: Settings' Install info line on iOS, for telling sideload installers apart from what they leave in our own
//! bundle: the keys Info.plist has beyond the ones `tools/ios-ipa.sh` writes, its URL schemes, the files beside the
//! .app and the signing profile's kind and team. Gathered on the phone by `ios::install_info`; the wording and the
//! profile's reading are here, so they're tested off the phone. Nothing is sent anywhere: the player screenshots it.

/// The Info.plist keys `tools/ios-ipa.sh` writes (top level and nested; a test keeps this in step with the script),
/// and the ones Foundation adds to the info dictionary as it loads it.
pub const OURS: &[&str] = &[
    "CFBundleExecutable",
    "CFBundleIdentifier",
    "CFBundleName",
    "CFBundleDisplayName",
    "CFBundlePackageType",
    "CFBundleShortVersionString",
    "CFBundleVersion",
    "CFBundleInfoDictionaryVersion",
    "CFBundleSupportedPlatforms",
    "DTPlatformName",
    "MinimumOSVersion",
    "UIDeviceFamily",
    "UIRequiredDeviceCapabilities",
    "CFBundleIcons",
    "CFBundlePrimaryIcon",
    "CFBundleIconFiles",
    "UIApplicationSceneManifest",
    "UIApplicationSupportsMultipleScenes",
    "UISceneConfigurations",
    "UIWindowSceneSessionRoleApplication",
    "UISceneConfigurationName",
    "UILaunchScreen",
    "UIRequiresFullScreen",
    "UIStatusBarHidden",
    "UISupportedInterfaceOrientations",
    "UISupportedInterfaceOrientations~ipad",
    "CADisableMinimumFrameDurationOnPhone",
    "UIFileSharingEnabled",
    "LSSupportsOpeningDocumentsInPlace",
    "NSLocalNetworkUsageDescription",
    // Foundation's own, not in the file
    "CFBundleInfoPlistURL",
    "CFBundleNumericVersion",
    "CFBundleExecutablePath",
    "NSBundleInitialPath",
    "NSBundleResolvedPath",
];

/// A signing profile (`embedded.mobileprovision`), as far as the Install info line names it.
#[derive(Debug, PartialEq)]
pub struct Profile {
    /// development, ad-hoc, enterprise or App Store.
    pub kind: &'static str,
    /// How many devices it lists (development and ad-hoc only).
    pub devices: usize,
    pub team: Option<String>,
    pub name: Option<String>,
    /// Its expiry day (YYYY-MM-DD).
    pub expires: Option<String>,
}

/// A signing profile from `embedded.mobileprovision`'s bytes: a signed envelope around an XML property list, which is
/// all that's read. Its kind: ProvisionsAllDevices is an enterprise profile; a device list is development when its
/// entitlements allow a debugger (get-task-allow), else ad-hoc; neither is App Store distribution.
pub fn profile(bytes: &[u8]) -> Option<Profile> {
    let start = find(bytes, b"<?xml").or_else(|| find(bytes, b"<plist"))?;
    let end = start + find(&bytes[start..], b"</plist>")? + "</plist>".len();
    let xml = std::str::from_utf8(&bytes[start..end]).ok()?;
    let devices = value(xml, "ProvisionedDevices").map(|v| v.matches("<string>").count());
    let kind = if value(xml, "ProvisionsAllDevices") == Some("true") {
        "enterprise"
    } else if devices.is_some() {
        if value(xml, "get-task-allow") == Some("true") { "development" } else { "ad-hoc" }
    } else {
        "App Store"
    };
    let text = |k| value(xml, k).filter(|v| !v.starts_with('<')).map(unescape);
    Some(Profile {
        kind,
        devices: devices.unwrap_or(0),
        team: text("TeamName"),
        name: text("Name"),
        expires: text("ExpirationDate").map(|d| d.chars().take(10).collect()),
    })
}

/// The first `needle` in `hay`.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Key `key`'s value in a property list: a string's, date's or number's words, `true`/`false`, or an array's or dict's
/// inside as it's written (the first key of that name anywhere; the profile's keys are unique).
fn value<'a>(xml: &'a str, key: &str) -> Option<&'a str> {
    let at = xml.find(&format!("<key>{key}</key>"))? + key.len() + "<key></key>".len();
    let rest = xml[at..].trim_start();
    if rest.starts_with("<true/>") {
        return Some("true");
    }
    if rest.starts_with("<false/>") {
        return Some("false");
    }
    let tag = &rest[1..rest.find('>')?];
    if tag.ends_with('/') {
        return Some("");
    }
    let inner = &rest[tag.len() + 2..];
    if tag == "array" || tag == "dict" {
        // up to its own closing tag, past any of the same nested in it
        let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
        let (mut depth, mut i) = (1, 0);
        while depth > 0 {
            let c = inner[i..].find(&close)? + i;
            depth += inner[i..c].matches(&open).count() as i32 - 1;
            i = c + close.len();
        }
        return Some(&inner[..i - close.len()]);
    }
    Some(&inner[..inner.find(&format!("</{tag}>"))?])
}

/// XML's five entities as their characters.
fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// What the probe found on the phone.
#[derive(Debug)]
pub struct Found {
    pub bundle_id: Option<String>,
    /// Info.plist's top-level keys that aren't [`OURS`].
    pub keys: Vec<String>,
    pub schemes: Vec<String>,
    /// The names beside the .app in its container (Err: the folder couldn't be listed, with the markers looked for
    /// one by one that are there).
    pub beside: Result<Vec<String>, Vec<String>>,
    /// embedded.mobileprovision: none, or unreadable (`Some(None)`).
    pub profile: Option<Option<Profile>>,
}

impl Default for Found {
    fn default() -> Self {
        Found { bundle_id: None, keys: vec![], schemes: vec![], beside: Ok(vec![]), profile: None }
    }
}

/// The Install info line ("Install info: ..."), sentences separated so it wraps anywhere.
pub fn line(f: &Found) -> String {
    let list = |v: &[String]| if v.is_empty() { "none".to_string() } else { v.join(", ") };
    let beside = match &f.beside {
        Ok(v) => list(v),
        Err(v) => format!("(can't list) {}", list(v)),
    };
    let profile = match &f.profile {
        None => "none".to_string(),
        Some(None) => "unreadable".to_string(),
        Some(Some(p)) => {
            let mut s = p.kind.to_string();
            if p.devices > 0 {
                s += &format!(", {} device{}", p.devices, if p.devices == 1 { "" } else { "s" });
            }
            if let Some(t) = &p.team {
                s += &format!(", team {t}");
            }
            if let Some(n) = &p.name {
                s += &format!(", named {n}");
            }
            if let Some(e) = &p.expires {
                s += &format!(", expires {e}");
            }
            s
        }
    };
    format!(
        "Install info: bundle {}. Added Info.plist keys: {}. URL schemes: {}. Beside the app: {}. Profile: {profile}.",
        f.bundle_id.as_deref().unwrap_or("unknown"),
        list(&f.keys),
        list(&f.schemes),
        beside,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped(dict: &str) -> Vec<u8> {
        let mut b = vec![0x30, 0x82, 0x1f, 0x00, 0x06, 0x09];
        b.extend(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\">\n<dict>{dict}</dict>\n</plist>").bytes());
        b.extend([0xa0, 0x82, 0x0d, 0xff]);
        b
    }

    #[test]
    fn profile_kinds() {
        let dev = wrapped(
            "<key>Entitlements</key><dict><key>application-identifier</key><string>T.x</string>\n\t<key>get-task-allow</key>\n\t<true/></dict>\
             <key>ExpirationDate</key><date>2026-11-05T10:00:00Z</date><key>Name</key><string>AltStore Profile</string>\
             <key>ProvisionedDevices</key><array><string>a</string><string>b</string></array><key>TeamName</key><string>Jo &amp; Co</string>",
        );
        assert_eq!(
            profile(&dev),
            Some(Profile { kind: "development", devices: 2, team: Some("Jo & Co".into()), name: Some("AltStore Profile".into()), expires: Some("2026-11-05".into()) })
        );
        let adhoc = wrapped("<key>Entitlements</key><dict><key>get-task-allow</key><false/></dict><key>ProvisionedDevices</key><array><string>a</string></array>");
        assert_eq!(profile(&adhoc).unwrap().kind, "ad-hoc");
        let ent = wrapped("<key>ProvisionsAllDevices</key><true/><key>TeamName</key><string>Corp</string>");
        assert_eq!(profile(&ent).unwrap().kind, "enterprise");
        assert_eq!(profile(&wrapped("<key>TeamName</key><string>Corp</string>")).unwrap().kind, "App Store");
        assert_eq!(profile(b"no plist here"), None);
    }

    #[test]
    fn nested_values() {
        let x = "<dict><key>A</key><dict><key>B</key><dict/><key>C</key><dict><key>D</key><string>d</string></dict></dict><key>E</key><string>e</string></dict>";
        assert_eq!(value(x, "A"), Some("<key>B</key><dict/><key>C</key><dict><key>D</key><string>d</string></dict>"));
        assert_eq!(value(x, "E"), Some("e"));
        assert_eq!(value(x, "Z"), None);
    }

    #[test]
    fn the_line() {
        let f = Found {
            bundle_id: Some("io.github.fizl0.hst.ABCDE".into()),
            keys: vec!["ALTDeviceID".into()],
            schemes: vec![],
            beside: Err(vec!["_TrollStore".into()]),
            profile: Some(Some(Profile { kind: "ad-hoc", devices: 1, team: None, name: None, expires: None })),
        };
        assert_eq!(
            line(&f),
            "Install info: bundle io.github.fizl0.hst.ABCDE. Added Info.plist keys: ALTDeviceID. URL schemes: none. Beside the app: (can't list) _TrollStore. Profile: ad-hoc, 1 device."
        );
    }

    /// Every key `tools/ios-ipa.sh` writes is in [`OURS`], so the line lists only what an installer added.
    #[test]
    fn ours_matches_the_ipa_script() {
        let script = include_str!("../../../tools/ios-ipa.sh");
        let keys: Vec<&str> = script.split("<key>").skip(1).filter_map(|s| s.split("</key>").next()).collect();
        assert!(keys.len() > 20);
        for k in keys {
            assert!(OURS.contains(&k), "{k} is written by tools/ios-ipa.sh but missing from OURS");
        }
    }
}

//! Online match setup (M2e): the host picks the match seed, and a joining peer's handshake (in its lobby Hello) must
//! agree with the host's on what the lobby can't make the same, its build and its disc, else the host refuses it with
//! a message saying what differs (M142), and on how it builds a mod's generated shot tables (M151d: `tables`). The rest that changes the simulation is the lobby's own: the rules, seats
//! and picks (B40g3: each player's handicap with them, the host's to set) come from the host's `Room`, a mod's
//! gameplay data from the wire (`mods_online::Wire`); anything that still drifts shows in the desync hash. M163b: the
//! disc is compared by what it plays (`hst_data::digest`, `Handshake::sim`), not its bytes, so releases and imports
//! whose discs play the same match meet.

use bevy::prelude::*;
use hst_sim::rng::Seed;

/// The match seed to play with, when it isn't this app's to pick (a peer takes the host's): `load` uses it instead
/// of picking one.
#[derive(Resource, Clone, Copy)]
pub struct MatchSeed(pub Seed);

/// What a peer brings to a lobby: its build and its disc.
#[derive(Clone, Debug, PartialEq)]
pub struct Handshake {
    pub build: String,
    /// The disc pack's hash (`Iso::pack_hash`); `None` running from an ISO (its disc ID is checked at start).
    pub pack: Option<u64>,
    /// M151d: this machine's fingerprint of the generated trajectory tables (`hst_sim::shot::probe`): a mod's §6
    /// `shots` `power`/`arc` tables are built by every peer from the wire, so they must come out the same. `None`
    /// from a peer that doesn't send it.
    pub tables: Option<u64>,
    /// M163b: its disc's gameplay digest (`hst_data::digest`): what on the disc feeds the match. When both sides send
    /// one it is checked instead of `pack`, so players on different releases (or imports) whose discs play the same
    /// match meet; `None` before the disc's digest could be read, or from a peer that doesn't send it.
    pub sim: Option<u64>,
    /// M171: its disc's digest piece by piece (`hst_data::digest::Manifest`): a peer whose disc plays differently from
    /// the host's is let in with it, and the host sends what its disc plays differently (`patch_for`). `None` with no
    /// digest, or from a peer that doesn't send it (refused on a digest that differs, as before).
    pub manifest: Option<hst_data::digest::Manifest>,
}

/// This machine's [`Handshake::tables`], worked out once (a few milliseconds).
pub fn tables() -> u64 {
    static TABLES: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *TABLES.get_or_init(hst_sim::shot::probe)
}

/// The build: two builds can't be trusted to simulate alike. M22: one id for the lobby's Hello (`net.rs`) and this
/// handshake, the crate version and the git commit ("1.0.0 2e28939", a "+" after it when built with uncommitted
/// changes); the game (`hst`, whose build script knows the commit) sets it first thing (`set_build`). Unset (this
/// crate's tests, `hst`'s dev builds) it is the version and "dev": dev builds of one version meet each other.
static BUILD: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// This build's id (see `BUILD`).
pub fn build() -> &'static str {
    BUILD.get().copied().unwrap_or(concat!(env!("CARGO_PKG_VERSION"), " dev"))
}

/// Stamps this build's id, before any lobby or match; a second call changes nothing.
pub fn set_build(id: &'static str) {
    let _ = BUILD.set(id);
}

/// This app's disc pack hash (`Handshake::pack`), stamped when the disc is opened (`set_pack`).
static PACK: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

/// This app's disc pack hash, `None` from an ISO or before a disc.
pub fn pack() -> Option<u64> {
    *PACK.lock().unwrap()
}

/// Stamps the disc this app plays (`Iso::pack_hash`), before any lobby.
pub fn set_pack(hash: Option<u64>) {
    *PACK.lock().unwrap() = hash;
}

/// The disc (ISO or pack path) whose gameplay digest `sim` reads, stamped with the pack (`set_disc`).
static DISC: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
/// The digest (and its manifest) worked out last and the disc it is of; held while one is being worked out, so a
/// second asker waits.
static SIM: std::sync::Mutex<Option<(String, Option<(u64, hst_data::digest::Manifest)>)>> = std::sync::Mutex::new(None);

/// Stamps the disc this app plays (M163b): its pack hash (`set_pack`) and, for `sim`, its path. On desktop the
/// digest (a few hundred milliseconds of reading) is worked out at once on a thread of its own, so a lobby has it ready.
pub fn set_disc(path: &str) {
    set_pack(hst_data::iso::Iso::open(path).ok().and_then(|i| i.pack_hash));
    *DISC.lock().unwrap() = Some(path.to_string());
    // M175: the Rules setting's other release over this disc, where one is chosen
    crate::rules_choice::apply();
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(sim);
}

/// M174: what was worked out of this app's disc (its digest and manifest, its rules) is worked out again: another
/// release's data was lent to it (`hst_data::crossplay::lend`), so it plays that release's game now.
pub fn forget_disc_data() {
    *SIM.lock().unwrap() = None;
    RULES_AGAIN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Bumped by [`forget_disc_data`]: `rules` reads the disc again.
static RULES_AGAIN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// M178's test: this app's disc's path, its pack hash left as it is (other tests in the binary stamp their own).
#[cfg(test)]
pub(crate) fn set_disc_path(path: &str) {
    *DISC.lock().unwrap() = Some(path.to_string());
}

/// The disc this app plays (`set_disc`'s path).
pub fn disc() -> Option<String> {
    DISC.lock().unwrap().clone()
}

/// This app's disc's gameplay digest (`Handshake::sim`), worked out once per disc; `None` without a disc or when it
/// can't be read (a release the remaster doesn't read).
pub fn sim() -> Option<u64> {
    survey().map(|(d, _)| d)
}

/// This app's disc's digest piece by piece (`Handshake::manifest`).
pub fn manifest() -> Option<hst_data::digest::Manifest> {
    survey().map(|(_, m)| m)
}

/// The disc's own digest and manifest, worked out once per disc: read without a lobby host's data over it
/// (`hst_data::crossplay`), so it is always what this disc itself plays.
fn survey() -> Option<(u64, hst_data::digest::Manifest)> {
    let path = disc()?;
    let mut cache = SIM.lock().unwrap();
    if let Some((p, d)) = cache.as_ref()
        && *p == path
    {
        return d.clone();
    }
    let d = hst_data::iso::Iso::open_own(&path).map_err(|e| e.to_string()).and_then(|mut iso| hst_data::digest::survey(|f| iso.read_unlogged(f).ok()));
    if let Err(e) = &d {
        eprintln!("hst: no gameplay digest for {path}: {e}");
    }
    *cache = Some((path, d.as_ref().ok().cloned()));
    d.ok()
}

/// M171: which release's rules a disc plays, from its `SYSTEM.CNF`: `"us"`, `"jp"`, `"eu"`; empty when unknown.
pub fn rules_of(system_cnf: &[u8]) -> &'static str {
    let text = String::from_utf8_lossy(system_cnf).to_ascii_uppercase();
    match () {
        _ if text.contains("SCUS_976") => "us",
        _ if text.contains("SCPS_151") => "jp",
        _ if text.contains("SCES_545") => "eu",
        _ => "",
    }
}

/// This app's disc's rules (`rules_of`), read off the disc itself; empty without a disc.
pub fn rules() -> &'static str {
    let Some(path) = disc() else { return "" };
    static RULES: std::sync::Mutex<Option<(String, u32, &'static str)>> = std::sync::Mutex::new(None);
    let mut c = RULES.lock().unwrap();
    let again = RULES_AGAIN.load(std::sync::atomic::Ordering::Relaxed);
    if let Some((p, n, r)) = c.as_ref()
        && *p == path
        && *n == again
    {
        return r;
    }
    let r = hst_data::iso::Iso::open_own(&path).ok().and_then(|mut i| i.read_unlogged("SYSTEM.CNF").ok()).map_or("", |c| rules_of(&c));
    *c = Some((path, again, r));
    r
}

/// M171: a release's rules in the player's words ("Japanese"), for the lobby list and card.
pub fn rules_name(rules: &str) -> &'static str {
    match rules {
        "us" => "US",
        "jp" => "Japanese",
        "eu" => "European",
        _ => "",
    }
}

/// M171: how the host's rules (`host`) play differently from this disc's own (`own`), in player terms, one line each;
/// empty when they play alike or it isn't known. From the Japanese build's rule list (`hst_sim::build::Build`) and the
/// discs' data (M163a); the European disc doesn't play yet (M174), so nothing is listed for it.
pub fn rules_differ(host: &str, own: &str) -> Vec<&'static str> {
    match (host, own) {
        ("jp", "us") => vec![
            "Every serve is timed like a rally shot.",
            "A mistimed strong serve never goes astray.",
            "No special serves, and no counter shots.",
            "Lobs and slices can't be aimed as wide.",
            "Smashes off their timing narrow only deep in the court.",
            "Computer players don't react extra to lobs.",
            "Some characters' shots fly differently.",
            "The red landing marker shows on serves too.",
        ],
        ("us", "jp") => vec![
            "Strong and weak serves have their own timing.",
            "A mistimed strong serve can go astray.",
            "Special serves and counter shots are in.",
            "Lobs and slices can be aimed wider.",
            "Smashes off their timing narrow from mid-court.",
            "Computer players react to lobs and special serves.",
            "Some characters' shots fly differently.",
        ],
        _ => Vec::new(),
    }
}

/// M171: the host's side: what a guest whose disc's manifest is `peer` needs of this app's disc to play the host's
/// match (`hst_data::crossplay::Patch`, encoded for the wire); `None` when it can't be read.
pub fn patch_for(peer: &hst_data::digest::Manifest) -> Option<Vec<u8>> {
    let path = disc()?;
    // M175: the Rules choice's other release where it is laid over the disc (what this host's lobby plays)
    let mut iso = if crate::rules_choice::laid() { hst_data::iso::Iso::open(&path) } else { hst_data::iso::Iso::open_own(&path) }.ok()?;
    let p = hst_data::crossplay::patch_for(|f| iso.read_unlogged(f).ok(), peer).map_err(|e| eprintln!("hst: no crossplay data: {e}")).ok()?;
    Some(p.encode())
}

/// M171: a guest's side: the host's data (`bytes`, from `patch_for`) laid over this app's disc and checked to play as
/// the host's digest `host`; `Ok` and installed until `end_crossplay`.
pub fn take_patch(bytes: &[u8], host: u64) -> Result<(), String> {
    let path = disc().ok_or("no disc")?;
    let p = hst_data::crossplay::Patch::decode(bytes).ok_or("the host's game data came damaged")?;
    hst_data::crossplay::verify(&path, p, host)
}

/// M171: leaving the host's lobby: this app's disc plays as itself again.
pub fn end_crossplay() {
    if let Some(path) = disc() {
        hst_data::crossplay::remove(path);
    }
}

/// A pack hash as a release's name, for messages ("SCPS-15113 Japan 1.00"), else the hash.
fn disc_name(pack: Option<u64>) -> String {
    match pack {
        Some(h) => hst_data::iso::release_name(h).map_or(format!("{h:016x}"), str::to_string),
        None => "a disc image".into(),
    }
}

impl Handshake {
    /// This app's.
    pub fn here() -> Handshake {
        let survey = survey();
        Handshake { build: build().into(), pack: pack(), tables: Some(tables()), sim: survey.as_ref().map(|s| s.0), manifest: survey.map(|s| s.1) }
    }

    /// A joining peer's handshake (`self`) against the host's: `Ok` to play, else why not, in words for the player.
    pub fn check(&self, host: &Handshake) -> Result<(), String> {
        if self.build != host.build {
            return Err(crate::net::version_differs(&host.build, &self.build));
        }
        // M163b: discs that play the same match meet, whatever release or import they are
        if let (Some(a), Some(b)) = (self.sim, host.sim) {
            // M171: crossplay: a peer that says what its disc plays gets the host's data for what differs
            if a != b && self.manifest.is_none() {
                return Err(format!(
                    "your disc ({}) plays differently from the host's ({}): its game data differs (digest {a:016x}, host {b:016x}), so you can't play together.",
                    disc_name(self.pack),
                    disc_name(host.pack)
                ));
            }
        }
        // an older import and a newer one of the same disc play together
        else if let (Some(a), Some(b)) = (self.pack, host.pack)
            && !hst_data::iso::same_disc(a, b)
        {
            return Err(format!(
                "your imported disc ({a:016x}) isn't the same as the host's ({b:016x}). Re-import an unmodified Hot Shots Tennis (USA) disc."
            ));
        }
        if let (Some(a), Some(b)) = (self.tables, host.tables)
            && a != b
        {
            return Err(format!("your device computes the game differently from the host's ({a:016x}, host {b:016x}): you can't play together."));
        }
        Ok(())
    }

    /// M171: a joining peer (`self`) whose disc plays differently from the host's, let in to play on the host's data
    /// (`check` passed): the host sends it what differs (`patch_for`).
    pub fn needs_data(&self, host: &Handshake) -> bool {
        matches!((self.sim, host.sim), (Some(a), Some(b)) if a != b) && self.manifest.is_some()
    }
}

/// A mod's gameplay data (what feeds the simulation: donor, hand, TParam row and overrides, AI row, §6 build) as a 64-bit
/// FNV-1a hash, the same on every platform. Looks (models, textures, faces, voice, motions drawn) aren't in it.
#[cfg(test)]
pub fn gameplay_hash(m: &crate::mods::Mod) -> u64 {
    let s = format!("{} {} {} {:?} {} {:?}", m.donor, m.hand.to_bits(), m.base, m.overrides, m.ai_row, m.build);
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::Mod;

    #[test]
    fn same_build_and_disc_join_and_differences_are_refused() {
        let h = Handshake { build: build().into(), pack: Some(7), tables: Some(tables()), sim: None, manifest: None };
        assert_eq!(h.check(&h), Ok(()));
        // an ISO (no pack hash) is taken on its disc ID, on either side
        assert_eq!(Handshake { pack: None, ..h.clone() }.check(&h), Ok(()));
        assert_eq!(h.check(&Handshake { pack: None, ..h.clone() }), Ok(()));
        let e = Handshake { build: "0.0.0 1a2b3c4".into(), ..h.clone() }.check(&h).unwrap_err();
        assert!(e.starts_with("a different version") && e.contains(&format!("host {}, you 0.0.0 1a2b3c4", build())), "{e}");
        let [old, new] = [0, 1].map(|i| Some(hst_data::iso::KNOWN_PACKS[i].0));
        assert_eq!(Handshake { pack: old, ..h.clone() }.check(&Handshake { pack: new, ..h.clone() }), Ok(()));
        let e = Handshake { pack: Some(8), ..h.clone() }.check(&h).unwrap_err();
        assert!(e.contains("imported disc (0000000000000008) isn't the same as the host's (0000000000000007)") && e.contains("Re-import"), "{e}");
        // M151d: the generated tables' fingerprint must agree (a peer without one is taken on its build)
        assert_eq!(Handshake::here().tables, Some(tables()));
        assert_eq!(Handshake { tables: None, ..h.clone() }.check(&h), Ok(()));
        let e = Handshake { tables: Some(tables() ^ 1), ..h.clone() }.check(&h).unwrap_err();
        assert!(e.contains("computes the game differently"), "{e}");
    }

    #[test]
    fn discs_that_play_alike_meet_whatever_release() {
        let [us, jp] = [0, 2].map(|i| Some(hst_data::iso::KNOWN_PACKS[i].0));
        let h = Handshake { build: build().into(), pack: us, tables: Some(tables()), sim: Some(1), manifest: None };
        // M163b: another release (or a modified import) with the same gameplay digest is let in
        assert_eq!(Handshake { pack: jp, ..h.clone() }.check(&h), Ok(()));
        assert_eq!(Handshake { pack: Some(8), ..h.clone() }.check(&h), Ok(()));
        // a different digest is refused, naming both releases, even from the same release's pack
        let e = Handshake { pack: jp, sim: Some(2), ..h.clone() }.check(&h).unwrap_err();
        assert!(e.contains("(SCPS-15113 Japan 1.00) plays differently from the host's (SCUS-97610 US 1.00)"), "{e}");
        assert!(Handshake { sim: Some(2), ..h.clone() }.check(&h).is_err());
        // without a digest on one side, the packs decide as before
        assert!(Handshake { pack: jp, sim: None, ..h.clone() }.check(&h).unwrap_err().contains("Re-import"));
        assert_eq!(Handshake { sim: None, ..h.clone() }.check(&h), Ok(()));
        // M171: a different digest with its manifest is let in, and the host sends it its data; the same one isn't sent any
        let m = Some(hst_data::digest::Manifest::default());
        let peer = Handshake { pack: jp, sim: Some(2), manifest: m.clone(), ..h.clone() };
        assert_eq!(peer.check(&h), Ok(()));
        assert!(peer.needs_data(&h));
        assert!(!Handshake { manifest: m, ..h.clone() }.needs_data(&h));
        assert!(!Handshake { sim: Some(2), ..h.clone() }.needs_data(&h));
    }

    /// M171: a disc's rules from its `SYSTEM.CNF`, their names, and how the other release's play differently.
    #[test]
    fn rules_and_how_they_differ() {
        assert_eq!(rules_of(b"BOOT2 = cdrom0:\\SCUS_976.10;1\r\nVER = 1.00"), "us");
        assert_eq!(rules_of(b"BOOT2 = cdrom0:\\SCPS_151.13;1"), "jp");
        assert_eq!(rules_of(b"BOOT2 = cdrom0:\\SCES_545.48;1"), "eu");
        assert_eq!(rules_of(b"BOOT2 = cdrom0:\\SLUS_000.00;1"), "");
        assert_eq!(["us", "jp", "eu", ""].map(rules_name), ["US", "Japanese", "European", ""]);
        assert!(["us", "jp", "eu", ""].iter().all(|r| rules_differ(r, r).is_empty()) && rules_differ("us", "eu").is_empty() && rules_differ("", "jp").is_empty());
        let (jp, us) = (rules_differ("jp", "us"), rules_differ("us", "jp"));
        assert!(jp.len() > 3 && us.len() > 3 && jp != us);
    }

    #[test]
    fn mod_hash_follows_gameplay_data_only() {
        let m = Mod {
            dir: "x".into(),
            id: "x".into(),
            name: "X".into(),
            source: String::new(),
            costumes: vec!["model/c00.glb".into()],
            donor: 3,
            hand: 1.0,
            base: 3,
            overrides: vec![("Strk POW".into(), "5".into())],
            ai_row: 3,
            texture_face: false,
            no_face: false,
            voice: "voice/".into(),
            motions: None,
            costume_of: None,
            skeleton: None,
            emotes: None, voice_takes: None,
            upscale: true,
            build: Default::default(),
        };
        let h = gameplay_hash(&m);
        assert_eq!(h, gameplay_hash(&Mod { name: "Y".into(), costumes: vec![], voice: "v/".into(), ..m.clone() }));
        assert_ne!(h, gameplay_hash(&Mod { hand: -1.0, ..m.clone() }));
        assert_ne!(h, gameplay_hash(&Mod { ai_row: 4, ..m.clone() }));
        assert_ne!(h, gameplay_hash(&Mod { overrides: vec![("Strk POW".into(), "6".into())], ..m.clone() }));
        assert_ne!(h, gameplay_hash(&Mod { build: crate::mods::Build { power: [1.4, 1.0, 1.0, 1.0], ..Default::default() }, ..m.clone() }));
    }
}

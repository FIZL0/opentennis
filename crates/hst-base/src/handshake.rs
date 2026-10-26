//! Online match setup (M2e): the host picks the match seed, and a joining peer's handshake (in its lobby Hello) must
//! agree with the host's on what the lobby can't make the same, its build and its disc, else the host refuses it with
//! a message saying what differs (M142). The rest that changes the simulation is the lobby's own: the rules, seats
//! and picks (B40g3: each player's handicap with them, the host's to set) come from the host's `Room`, a mod's
//! gameplay data from the wire (`mods_online::Wire`); anything that still drifts shows in the desync hash.

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

impl Handshake {
    /// This app's.
    pub fn here() -> Handshake {
        Handshake { build: build().into(), pack: pack() }
    }

    /// A joining peer's handshake (`self`) against the host's: `Ok` to play, else why not, in words for the player.
    pub fn check(&self, host: &Handshake) -> Result<(), String> {
        if self.build != host.build {
            return Err(crate::net::version_differs(&host.build, &self.build));
        }
        // an older import and a newer one of the same disc play together
        if let (Some(a), Some(b)) = (self.pack, host.pack)
            && !hst_data::iso::same_disc(a, b)
        {
            return Err(format!(
                "your imported disc ({a:016x}) isn't the same as the host's ({b:016x}). Re-import an unmodified Hot Shots Tennis (USA) disc."
            ));
        }
        Ok(())
    }
}

/// A mod's gameplay data (what feeds the simulation: donor, hand, TParam row and overrides, AI row) as a 64-bit
/// FNV-1a hash, the same on every platform. Looks (models, textures, faces, voice, motions drawn) aren't in it.
#[cfg(test)]
pub fn gameplay_hash(m: &crate::mods::Mod) -> u64 {
    let s = format!("{} {} {} {:?} {}", m.donor, m.hand.to_bits(), m.base, m.overrides, m.ai_row);
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::Mod;

    #[test]
    fn same_build_and_disc_join_and_differences_are_refused() {
        let h = Handshake { build: build().into(), pack: Some(7) };
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
    }
}

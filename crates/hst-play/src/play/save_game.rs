//! M193: match saves. A save is the match's setup (who sits where, the characters, costumes, hands, handicaps, court,
//! umpire, length, the mods with their versions, the rules and the match rate), the game build and the mode, plus the
//! match state from the start of the point being played (`snapshot::encode`, the state a netplay resync sends:
//! generators included, so the match goes on exactly as it would have).
//!
//! While a match plays, [`PointStart`] keeps the latest point's start as the instant replay's serve set-up kept it
//! (only read, so a match that's never saved plays as before). [`capture`] turns it into a [`MatchSave`]; [`Saves`]
//! keeps them in the user data dir, one per line-up and mode (a new one replaces it), the newest [`KEEP`]. To resume:
//! [`Head::check`] (the build, rules and mods must be the ones it was saved with), [`Setup::insert`] (or the save's
//! own [`Setup::flags`] through the menus' match start), the match's setup, then [`resume`].

use super::{Game, Pads, instant_replay, snapshot};
use crate::mods::MatchMod;
use crate::mods_court::MatchCourt;
use crate::mods_umpire::MatchUmpire;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Saves kept, newest first.
pub const KEEP: usize = 5;

/// A save file's first bytes.
const MAGIC: &[u8; 8] = b"HSTSAVE1";

pub fn plugin(app: &mut App) {
    app.init_resource::<PointStart>().add_systems(FixedUpdate, keep.after(instant_replay::run));
    app.add_systems(PreUpdate, resume_next);
}

/// M195: a save the menu chose to resume, put into the next match once it's set up ([`resume_next`]).
static NEXT: std::sync::Mutex<Option<MatchSave>> = std::sync::Mutex::new(None);

/// M195: the next match to start resumes `save` (the menu starts it from the save's setup).
pub fn resume_in_next(save: MatchSave) {
    *NEXT.lock().unwrap_or_else(|e| e.into_inner()) = Some(save);
}

/// M195: once the match is set up (after every start-up system), the save waiting for it goes in.
fn resume_next(w: &mut World) {
    if !w.contains_resource::<Game>() {
        return;
    }
    let Some(save) = NEXT.lock().unwrap_or_else(|e| e.into_inner()).take() else { return };
    if let Err(e) = resume(w, &save) {
        warn!("couldn't resume the saved match: {e}");
    }
}

/// The latest point's start outside an instant replay (a replay puts an older point's start back: a save then
/// still picks up from the newest point).
#[derive(Resource, Default)]
pub struct PointStart(Option<Arc<snapshot::Snapshot>>);

/// After each tick: the serve set-up's state, once it's a new one and no replay plays.
fn keep(w: &mut World) {
    let Some(g) = w.get_resource::<Game>() else { return };
    if g.instant.replaying || instant_replay::playing(w) {
        return;
    }
    let Some(s) = instant_replay::serve(w) else { return };
    let mut k = w.resource_mut::<PointStart>();
    if !k.0.as_ref().is_some_and(|o| Arc::ptr_eq(o, &s)) {
        k.0 = Some(s);
    }
}

/// Where the match is played.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Local,
    Online,
}

/// A mod in the match, as installed when it was saved.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ModPin {
    /// Its folder's name (`mods::Mod::folder`).
    pub folder: String,
    pub dir: PathBuf,
    /// Its `mod.json` version (`modstore::version`).
    pub version: String,
}

impl ModPin {
    pub fn of(dir: &Path) -> Self {
        let folder = dir.file_name().map_or_else(|| dir.display().to_string(), |n| n.to_string_lossy().to_string());
        ModPin { folder, dir: dir.to_path_buf(), version: crate::modstore::version(dir) }
    }

    /// Still installed, at the same version.
    pub fn unchanged(&self) -> bool {
        crate::modfs::is_file(&self.dir.join("mod.json")) && crate::modstore::version(&self.dir) == self.version
    }
}

/// What the match was set up with.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Setup {
    pub singles: bool,
    /// Characters and costumes (the costume is also a computer player's strength), by match player.
    pub chars: Vec<usize>,
    pub outfits: Vec<usize>,
    /// Seats 1P..4P with a person at them (the rest are the computer).
    pub humans: [bool; 4],
    /// The hand toggle and handicap, by match player.
    pub swap_hands: [bool; 4],
    pub handicaps: [u8; 4],
    pub court: usize,
    pub stage: Option<u32>,
    pub umpire: u8,
    pub sets: i32,
    pub games: i32,
    /// Character mods by match player, and the court's and umpire's.
    pub mods: Vec<Option<ModPin>>,
    pub court_mod: Option<ModPin>,
    pub umpire_mod: Option<ModPin>,
    /// The release whose rules it plays (`rules_choice::chosen`) and its ticks a second.
    pub rules: u8,
    pub hz: u32,
    /// The flags the match was started with (the menus' match start), when the caller gave them.
    pub flags: Vec<String>,
}

impl Setup {
    /// The match in `w` as it was set up.
    pub fn of(w: &World, flags: Vec<String>) -> Option<Self> {
        let args = w.get_resource::<crate::Args>()?;
        let n = w.get_resource::<Game>()?.players.len();
        let mut humans = [false; 4];
        // the seats as set (autoplay plays them all, but they stay the line-up's), else who the pads made human
        match w.get_resource::<Pads>() {
            Some(Pads { seats: Some(s), .. }) => humans = *s,
            Some(p) => (0..n).filter_map(|i| p.slot_of(i, n)).for_each(|s| humans[s] = true),
            None => {}
        }
        let mods = w.get_resource::<MatchMod>().map_or_else(Vec::new, |m| m.0.iter().map(|m| m.as_ref().map(|m| ModPin::of(&m.dir))).collect());
        Some(Setup {
            singles: args.singles,
            chars: args.chars.clone(),
            outfits: args.outfits.clone(),
            humans,
            swap_hands: w.get_resource::<super::hand_swap::SwapHands>().map_or([false; 4], |s| s.0),
            handicaps: w.get_resource::<super::handicap::Handicaps>().map_or([0; 4], |h| h.0),
            court: args.court,
            stage: args.stage,
            umpire: args.umpire,
            sets: args.sets,
            games: args.games,
            mods,
            court_mod: w.get_resource::<MatchCourt>().map(|c| ModPin::of(&c.0.dir)),
            umpire_mod: w.get_resource::<MatchUmpire>().map(|u| ModPin::of(&u.0.dir)),
            rules: crate::rules_choice::chosen(),
            hz: w.get_resource::<crate::match_speed::MatchHz>().map_or(60, |h| h.0),
            flags,
        })
    }

    /// Who plays it: one save per line-up and mode.
    pub fn line_up(&self) -> String {
        let mods: Vec<_> = self.mods.iter().map(|m| m.as_ref().map_or("", |m| m.folder.as_str())).collect();
        format!("{} {:?} {:?} {:?} {:?}", if self.singles { "singles" } else { "doubles" }, self.humans, self.chars, self.outfits, mods)
    }

    /// Every mod it needs, installed at the version it was saved with.
    pub fn mods_unchanged(&self) -> bool {
        self.mods.iter().flatten().chain(&self.court_mod).chain(&self.umpire_mod).all(ModPin::unchanged)
    }

    /// The resources the match's setup reads, for this line-up (`base`: the disc and what a save doesn't hold).
    /// Mods are read from their folders (a changed one refuses: [`Head::check`] first).
    pub fn insert(&self, w: &mut World, base: &crate::Args) -> Result<(), String> {
        let mut mm = MatchMod::default();
        for (i, m) in self.mods.iter().enumerate() {
            if let Some(m) = m {
                mm.set(i, crate::mods::read(&m.dir)?);
            }
        }
        if !mm.0.is_empty() {
            w.insert_resource(mm);
        }
        if let Some(c) = &self.court_mod {
            w.insert_resource(MatchCourt(crate::mods_court::read(&c.dir)?));
        }
        if let Some(u) = &self.umpire_mod {
            w.insert_resource(MatchUmpire(crate::mods_umpire::read(&u.dir)?));
        }
        let args = crate::Args {
            singles: self.singles,
            chars: self.chars.clone(),
            outfits: self.outfits.clone(),
            court: self.court,
            stage: self.stage,
            umpire: self.umpire,
            sets: self.sets,
            games: self.games,
            play: true,
            iso: base.iso.clone(),
            archives: base.archives.clone(),
            shot: base.shot.clone(),
            shot_at: base.shot_at,
            radius: base.radius,
            ball: base.ball,
            viewer: None,
            sound: base.sound.clone(),
            music: base.music,
            pads: base.pads,
        };
        w.insert_resource(args);
        let pads = w.get_resource::<Pads>().map(|p| Pads { seats: Some(self.humans), ..p.clone() });
        w.insert_resource(pads.unwrap_or_else(|| Pads { seats: Some(self.humans), ..default() }));
        w.insert_resource(super::hand_swap::SwapHands(self.swap_hands));
        w.insert_resource(super::handicap::Handicaps(self.handicaps));
        Ok(())
    }
}

/// The score where it picks up, for a save's card.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Summary {
    pub sets: [i32; 2],
    pub games: [i32; 2],
    pub points: [i32; 2],
    pub tiebreak: bool,
    pub deuce: bool,
    pub advantage: bool,
    /// The player serving.
    pub server: i32,
    /// M195: each finished set's games, by team.
    pub set_games: Vec<[i32; 2]>,
}

impl Summary {
    /// Team `t`'s points as called: 0 15 30 40, AD on advantage (a tiebreak's as counted).
    pub fn call(&self, t: usize) -> String {
        let p = self.points[t];
        match p {
            _ if self.tiebreak => p.to_string(),
            0..=3 => ["0", "15", "30", "40"][p as usize].to_string(),
            _ => "AD".into(),
        }
    }

    /// The points as "30-15" (team 0's first).
    pub fn points_line(&self) -> String {
        format!("{}-{}", self.call(0), self.call(1))
    }

    /// The finished sets and this one's games, "6-4 4-2".
    pub fn games_line(&self) -> String {
        self.set_games.iter().chain([&self.games]).map(|g| format!("{}-{}", g[0], g[1])).collect::<Vec<_>>().join(" ")
    }
}

/// What a save's card and its check read (any build's file reads this far).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Head {
    /// The game build it was saved in (`handshake::build`).
    pub build: String,
    pub mode: Mode,
    /// Seconds since 1970 when it was saved.
    pub saved: u64,
    pub setup: Setup,
    pub summary: Summary,
}

/// Why a save won't load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Saved in another build (its version): the state layout can change between builds.
    Build(String),
    /// A mod it played with is gone or has another version.
    Mods,
    /// Saved under another release's rules or match rate.
    Rules,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Refusal::Build(v) => write!(f, "Saved in v{v}"),
            Refusal::Mods => f.write_str("Mods changed"),
            Refusal::Rules => f.write_str("Rules changed"),
        }
    }
}

impl Head {
    /// Whether this build, with the mods installed now, can resume it.
    pub fn check(&self) -> Result<(), Refusal> {
        self.check_in(crate::handshake::build(), crate::rules_choice::chosen(), crate::match_speed::MatchHz::now().0)
    }

    fn check_in(&self, build: &str, rules: u8, hz: u32) -> Result<(), Refusal> {
        if self.build != build {
            // the version, or the whole id when only the commit differs
            let v = self.build.split(' ').next().unwrap_or_default();
            let ours = build.split(' ').next().unwrap_or_default();
            return Err(Refusal::Build(if v == ours { self.build.clone() } else { v.to_string() }));
        }
        if self.setup.rules != rules || self.setup.hz != hz {
            return Err(Refusal::Rules);
        }
        if !self.setup.mods_unchanged() {
            return Err(Refusal::Mods);
        }
        Ok(())
    }

    /// Its file's name in [`Saves`]: one per line-up and mode.
    fn file(&self) -> String {
        format!("{:016x}.sav", snapshot::fnv(&format!("{:?} {}", self.mode, self.setup.line_up())))
    }
}

/// A saved match.
#[derive(Clone, Debug)]
pub struct MatchSave {
    pub head: Head,
    /// `snapshot::encode` of the state at the point's start.
    pub state: Vec<u8>,
}

impl MatchSave {
    pub fn to_bytes(&self) -> Vec<u8> {
        let head = serde_json::to_vec(&self.head).unwrap();
        [&MAGIC[..], &(head.len() as u32).to_le_bytes(), &head, &self.state].concat()
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, String> {
        let (head, rest) = split(b)?;
        Ok(MatchSave { head: serde_json::from_slice(head).map_err(|e| format!("not a save: {e}"))?, state: rest.to_vec() })
    }

    /// Its head only (a file of any build).
    pub fn head_of(b: &[u8]) -> Result<Head, String> {
        serde_json::from_slice(split(b)?.0).map_err(|e| format!("not a save: {e}"))
    }
}

fn split(b: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let rest = b.strip_prefix(&MAGIC[..]).ok_or("not a save")?;
    let n = rest.get(..4).ok_or("not a save")?;
    let n = u32::from_le_bytes(n.try_into().unwrap()) as usize;
    let rest = &rest[4..];
    if rest.len() < n {
        return Err("a save cut short".into());
    }
    Ok(rest.split_at(n))
}

/// The match in `w` as a save: its setup and the state at the current point's start. `None` with no match, before
/// its first point is set up, or once it's over.
pub fn capture(w: &mut World, mode: Mode, flags: Vec<String>) -> Option<MatchSave> {
    let setup = Setup::of(w, flags)?;
    if w.resource::<Game>().score.match_over {
        return None;
    }
    let start = if w.resource::<Game>().instant.replaying || instant_replay::playing(w) { w.get_resource::<PointStart>().and_then(|k| k.0.clone()) } else { instant_replay::serve(w) }?;
    // the state as the point started: put back, encoded, and the match as it is now put back again
    let now = snapshot::save(w);
    snapshot::restore(w, &start);
    let s = &w.resource::<Game>().score;
    let set_games = (0..(s.set.max(0) as usize).min(s.set_games[0].len())).map(|k| [s.set_games[0][k], s.set_games[1][k]]).collect();
    let summary = Summary { sets: s.sets, games: s.games, points: s.points, tiebreak: s.tiebreak, deuce: s.deuce, advantage: s.advantage, server: s.server, set_games };
    let state = snapshot::encode(w);
    snapshot::restore(w, &now);
    let saved = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    Some(MatchSave { head: Head { build: crate::handshake::build().to_string(), mode, saved, setup, summary }, state })
}

/// Puts a save's state into the match `w` just set up from its [`Setup`]: the match picks up at the point's start.
pub fn resume(w: &mut World, save: &MatchSave) -> Result<(), String> {
    snapshot::decode(w, &save.state)?;
    // the point's start is the serve set-up the instant replay and the next save go from
    let s = Arc::new(snapshot::save(w));
    instant_replay::set_serve(w, Some(s.clone()));
    if let Some(mut k) = w.get_resource_mut::<PointStart>() {
        k.0 = Some(s);
    }
    Ok(())
}

/// A save on disk.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub head: Head,
}

/// The saves folder.
pub struct Saves {
    dir: PathBuf,
}

impl Saves {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Saves { dir: dir.into() }
    }

    /// The user's: `saves/` in the game's data folder (the browser has none yet: M198 keeps them in its storage).
    pub fn user() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        return Saves::new(crate::disc::data_dir().join("saves"));
        #[cfg(target_arch = "wasm32")]
        Saves::new("saves")
    }

    /// Writes `s` in place of its line-up's save, then drops all but the newest [`KEEP`].
    pub fn write(&self, s: &MatchSave) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("can't make {}: {e}", self.dir.display()))?;
        let path = self.dir.join(s.head.file());
        let part = path.with_extension("part");
        std::fs::write(&part, s.to_bytes()).and_then(|()| std::fs::rename(&part, &path)).map_err(|e| format!("can't write {}: {e}", path.display()))?;
        for old in self.list().into_iter().skip(KEEP) {
            self.delete(&old.path);
        }
        Ok(path)
    }

    /// The saves, newest first (files that aren't saves left out).
    pub fn list(&self) -> Vec<Entry> {
        let mut out: Vec<Entry> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "sav"))
            .filter_map(|path| Some(Entry { head: MatchSave::head_of(&std::fs::read(&path).ok()?).ok()?, path }))
            .collect();
        out.sort_by(|a, b| b.head.saved.cmp(&a.head.saved).then_with(|| a.path.cmp(&b.path)));
        out
    }

    pub fn read(&self, path: &Path) -> Result<MatchSave, String> {
        MatchSave::from_bytes(&std::fs::read(path).map_err(|e| format!("can't read {}: {e}", path.display()))?)
    }

    pub fn delete(&self, path: &Path) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::{Args, GameSpace};

    fn head(chars: Vec<usize>, saved: u64) -> Head {
        let setup = Setup { singles: true, chars, outfits: vec![0, 0], humans: [true, false, false, false], sets: 1, games: 4, hz: 60, ..default() };
        Head { build: "1.7.4 abc".into(), mode: Mode::Local, saved, setup, summary: Summary::default() }
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hst-save-game-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_line_up_keeps_one_save_and_the_newest_five_stay() {
        let dir = scratch("keep");
        let saves = Saves::new(&dir);
        for (i, c) in (0..7).enumerate() {
            saves.write(&MatchSave { head: head(vec![c, 1], 100 + i as u64), state: vec![i as u8; 3] }).unwrap();
        }
        // the same line-up again replaces its save
        saves.write(&MatchSave { head: head(vec![6, 1], 200), state: vec![9] }).unwrap();
        let list = saves.list();
        assert_eq!(list.len(), KEEP);
        let saved: Vec<_> = list.iter().map(|e| e.head.saved).collect();
        assert_eq!(saved, [200, 105, 104, 103, 102]);
        let s = saves.read(&list[0].path).unwrap();
        assert_eq!((s.head, s.state), (head(vec![6, 1], 200), vec![9]));
        // another mode is another save
        let mut online = head(vec![6, 1], 300);
        online.mode = Mode::Online;
        saves.write(&MatchSave { head: online, state: vec![] }).unwrap();
        assert_eq!(saves.list().iter().filter(|e| e.head.setup.chars == [6, 1]).count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refusals_say_why() {
        let h = head(vec![6, 1], 0);
        assert_eq!(h.check_in("1.7.4 abc", 0, 60), Ok(()));
        assert_eq!(h.check_in("1.7.5 def", 0, 60).unwrap_err().to_string(), "Saved in v1.7.4");
        assert_eq!(h.check_in("1.7.4 def", 0, 60).unwrap_err().to_string(), "Saved in v1.7.4 abc");
        assert_eq!(h.check_in("1.7.4 abc", 1, 60), Err(Refusal::Rules));
        assert_eq!(h.check_in("1.7.4 abc", 0, 50), Err(Refusal::Rules));
        // a mod at another version, then gone
        let dir = scratch("mods");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.json"), r#"{"version":"2"}"#).unwrap();
        let mut h = h;
        h.setup.mods = vec![None, Some(ModPin::of(&dir))];
        assert_eq!(h.check_in("1.7.4 abc", 0, 60), Ok(()));
        std::fs::write(dir.join("mod.json"), r#"{"version":"3"}"#).unwrap();
        assert_eq!(h.check_in("1.7.4 abc", 0, 60).unwrap_err().to_string(), "Mods changed");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(h.check_in("1.7.4 abc", 0, 60), Err(Refusal::Mods));
        // not a save, or one cut short
        assert!(MatchSave::from_bytes(b"hello").is_err());
        let b = MatchSave { head: head(vec![6, 1], 0), state: vec![1, 2] }.to_bytes();
        assert!(MatchSave::from_bytes(&b[..20]).is_err());
    }

    /// A headless match set up from `setup` (the match's own modules, no window or sound).
    fn app(s: &Setup) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        let base = Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 1, pads: None };
        s.insert(app.world_mut(), &base).unwrap();
        // another test may have turned autoplay on for the whole run: the recording's pads play 1P
        app.world_mut().resource_mut::<Pads>().autoplay = false;
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin));
        app.add_systems(PostStartup, super::super::setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain().before(instant_replay::run));
        app.add_plugins((instant_replay::plugin, super::plugin));
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        app
    }

    fn tick(app: &mut App, pad: [hst_sim::replay::Pad; 4]) -> u64 {
        app.insert_resource(tick_input::Replay([pad].into()));
        app.world_mut().run_schedule(FixedUpdate);
        snapshot::state_hash(app.world_mut())
    }

    /// The point's start kept now (its `Arc`'s address tells a new one).
    fn kept(app: &App) -> usize {
        app.world().resource::<PointStart>().0.as_ref().map_or(0, |s| Arc::as_ptr(s) as usize)
    }

    /// A match saved mid-point, through the file, into a fresh match: from the point's start, the same pads give the
    /// same state every tick as the match that went on.
    #[test]
    fn save_game() {
        let (Ok(rec), true) = (hst_data::fixture::read("fixtures/round1.bin"), hst_data::fixture::has_iso()) else {
            return;
        };
        let pads: Vec<_> = hst_sim::replay::frames(&rec).iter().map(|f| [tick_input::recorded(f.pad(0)), tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]).collect();
        let setup = Setup { singles: true, chars: vec![6, 1], outfits: vec![], humans: [true, false, false, false], court: 0, umpire: 4, sets: 1, games: 4, hz: 60, ..default() };
        let mut a = app(&setup);
        // the hashes after each tick, and the tick each point's start was kept after
        let (mut hashes, mut starts) = (vec![], vec![]);
        let mut last = kept(&a);
        for p in &pads {
            hashes.push(tick(&mut a, *p));
            if kept(&a) != last {
                last = kept(&a);
                starts.push(hashes.len() - 1);
            }
            // a later point, well into it
            if starts.len() >= 2 && hashes.len() > starts[starts.len() - 1] + 200 {
                break;
            }
        }
        let at = *starts.last().unwrap();
        assert!(at > 0 && hashes.len() > at + 200, "the recording reached a later point well in ({starts:?})");
        let save = capture(a.world_mut(), Mode::Local, vec![]).unwrap();
        assert_eq!(save.head.setup.chars, setup.chars);
        assert_eq!(save.head.setup.humans, setup.humans);
        // saving changed nothing in the match
        let n = 2400;
        let now = snapshot::state_hash(a.world_mut());
        assert_eq!(now, *hashes.last().unwrap());
        let mut on: Vec<u64> = hashes[at..].to_vec();
        while on.len() < n + 1 {
            let p = pads[at + on.len()];
            on.push(tick(&mut a, p));
        }

        let dir = scratch("resume");
        let saves = Saves::new(&dir);
        let path = saves.write(&save).unwrap();
        let back = saves.read(&path).unwrap();
        assert_eq!(back.head.check_in(crate::handshake::build(), crate::rules_choice::chosen(), 60), Ok(()));
        let mut b = app(&back.head.setup);
        resume(b.world_mut(), &back).unwrap();
        assert_eq!(snapshot::state_hash(b.world_mut()), on[0], "the save's state is the point's start");
        let again: Vec<u64> = (1..=n).map(|k| tick(&mut b, pads[at + k])).collect();
        for (k, (x, y)) in on[1..].iter().zip(&again).enumerate() {
            assert_eq!(x, y, "tick {k} after the point's start differs");
        }
        assert!(again.windows(2).any(|w| w[0] != w[1]), "the state never changed");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

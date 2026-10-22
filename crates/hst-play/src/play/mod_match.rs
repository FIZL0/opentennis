//! A standard mod (`crate::mods`) in a match slot: the donor's motions, trajectory tables, shot records, serve
//! and timing tables (the setup loads them by the donor's number), with the mod's own TParam.csv row (stats, aim,
//! reach and heights, smash window, power, timing stats, collision size), `ai_row` for the computer's
//! AIParam.csv row and second-toss pick, `hand`, and its voice bank (only its own takes, never the donor's: B181).

use std::sync::Arc;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use hst_data::iso::Iso;

use super::Game;
use crate::audio::SoundBank;
use crate::character::CharacterData;
use crate::mods::{self, MatchMod, Mod};

/// The mod's costume `outfit` (wrapping round its costume list).
pub(super) fn load(
    iso: &mut Iso,
    m: &Mod,
    outfit: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Arc<CharacterData> {
    let mut d = if m.costumes.is_empty() {
        // an online mod whose looks this machine declined (M2i): its donor, hue-shifted (B641: never taken for a
        // stock character)
        let d = crate::preload::load_disc(iso, m.donor, outfit, meshes, materials, images, bindposes)
            .or_else(|_| crate::character::load_disc(iso, m.donor, 0, meshes, materials, images, bindposes))
            .unwrap_or_else(|e| panic!("mod {} stand-in: {e}", m.id));
        crate::mods_online::tint(&d, materials, images);
        d
    } else {
        crate::preload::load_mod(iso, m, outfit % m.costumes.len(), meshes, materials, images, bindposes).unwrap_or_else(|e| panic!("mod {}: {e}", m.id))
    };
    if let Some(sk) = &m.skeleton {
        // online: every peer poses the owner's skeleton for the game (contacts, toss, body hits), whatever it draws
        let mo = crate::character::disc_motions(iso, m.donor, sk, |_| None).unwrap_or_else(|e| panic!("mod {}: {e}", m.id));
        if d.visual.is_empty() {
            d.visual = std::mem::take(&mut d.motions);
        }
        (d.motions, d.paths, d.pelvis, d.arm, d.stance_ball, d.skeleton) = (mo.motions, mo.paths, mo.pelvis, mo.arm, mo.stance_ball, (**sk).clone());
        // B556: what it draws differs by peer, so the cameras frame it as the game plays it
        d.frame_gameplay = true;
        d.emote_ticks = m.emotes.iter().flatten().cloned().collect();
    }
    // B596: its emotes cycle over its own takes, the owner's count online (the same on every peer)
    d.voice_takes = mods::voice_takes(m).into_iter().collect();
    Arc::new(d)
}

/// Player `i`'s TParam.csv row: the mod's in its slot, else character `c`'s.
pub(super) fn row(iso: &mut Iso, mm: Option<&MatchMod>, i: usize, c: usize) -> Vec<String> {
    match mm.and_then(|m| m.get(i)) {
        Some(m) => mods::tparam(iso, m).unwrap_or_else(|e| panic!("mod {}: {e}", m.id)),
        None => super::tparam(iso, c),
    }
}

/// Player `i`, set up as the mod's donor, takes the mod's row, AI row, hand and voices.
pub(super) fn apply(g: &mut Game, i: usize, iso: &mut Iso, m: &Mod, outfit: usize, voices: &mut [Option<Arc<SoundBank>>]) {
    let row = mods::tparam(iso, m).unwrap_or_else(|e| panic!("mod {}: {e}", m.id));
    let cells = |k: usize| -> Vec<f32> { row[k].split('/').map(|v| v.trim().parse().expect("TParam number")).collect() };
    let doubles = g.players.len() == 4;
    let p = &mut g.players[i];
    p.hand = m.hand;
    // a mod's §6 `run_speed` paces its run (1: the row's own speed)
    p.stats = super::stats_of(&row).paced(m.build.run);
    p.body.stamina = p.stats.stamina;
    // ponytail: the AI row's costume level is the slot's `--outfits` number, as a disc character's
    p.ai = super::ai_params(iso, m.ai_row, outfit, doubles);
    p.ai_second = super::ai_second_toss(iso, m.ai_row);
    g.aim_stats[i] = super::aim_of(&row);
    g.finish.power[i] = [13, 14].map(|k| row[k].parse().expect("TParam power"));
    let smash = cells(64);
    g.smash_heights[i] = [smash[0] / 100.0, smash[1] / 100.0];
    g.reaches[i] = super::reach_of(&g.reach, &row, m.hand);
    g.under_min[i] = hst_sim::player::ReachStats::from_tparam(&row.join(",")).under_min;
    g.timing[i].restat(&row);
    let n = g.players.len();
    g.swing_pace.resize(n, 1.0);
    g.swing_pace[i] = m.build.swing;
    // B181: a mod character speaks only its own takes; a program it has no wav for, or a mod with no voice, is silent
    // (never its donor's). A costume pack is its disc character and keeps that voice, as a disc-body mod its donor's (M11).
    if m.disc_voice().is_none() {
        voices[i] = mods::voice(m).map(Arc::new);
    }
}

/// A bot singles match with a rerigged mod (local `context/mods/test_pc00`, see the M1a journal) on court: the mod's
/// player gets its row, hand and AI and plays a rally (needs the ISO at the repository root; skipped without it).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Args, GameSpace};
    use std::path::Path;

    const ISO: &str = hst_data::fixture::ISO_PATH;
    /// The converted mod (`context/mods/test_pc00`, from another checkout when this one lacks it).
    fn mod_dir() -> std::path::PathBuf {
        hst_data::fixture::path("context/mods/test_pc00")
    }

    #[test]
    fn bot_match_with_a_mod_plays_a_rally() {
        if !hst_data::fixture::has_iso() || !hst_data::fixture::exists("context/mods/test_pc00") {
            return;
        }
        let m = mods::read(&mod_dir()).unwrap();
        let mut app = bot_match(&m, 1);
        use super::super::*;
        let mut iso = hst_data::fixture::iso().unwrap();
        let row = mods::tparam(&mut iso, &m).unwrap();
        {
            let g = app.world().resource::<Game>();
            let p = &g.players[1];
            assert_eq!((g.chars[1], p.hand, p.stats.speed), (0, -1.0, stats_of(&row).speed));
            assert_eq!(g.reaches[1].reach, loco::ReachStats::from_tparam(&row.join(",")).reach);
            assert_ne!(g.reaches[1].reach, character_reach(&g.reach, &mut iso, 0, -1.0).reach, "the override didn't reach the contact search");
            assert_eq!(g.data[1].joints.len(), mods::load(&mut iso, &m, 0, &mut Assets::<Mesh>::default(), &mut Assets::<StandardMaterial>::default(), &mut Assets::<Image>::default(), &mut Assets::<SkinnedMeshInverseBindposes>::default()).unwrap().joints.len());
        }
        rally(&mut app, &m);
    }

    /// M11: Bigger Chief (a disc-body mod of pc07, `scale` 2, `swing_speed` 2, `run_speed` 0.25) in a bot match: his
    /// body is pc07's twice as tall, his size-bound TParam cells (reach, hit heights) doubled, his run speed a quarter of
    /// pc07's, his swings paced at 2, and he plays (serves and returns balls).
    #[test]
    fn bigger_chief_plays() {
        if !hst_data::fixture::has_iso() {
            return hst_data::fixture::skip("no ISO");
        }
        let dir = std::env::temp_dir().join(format!("hst-bigger-chief-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.json"), r#"{"standard":1,"id":"hst_bigger_chief","name":"Bigger Chief","disc_body":true,"donor":7,"hand":"right","params":{"base":7,"override":{}},"ai_row":7,"face":"morph","scale":2,"swing_speed":2,"run_speed":0.25,"version":1}"#).unwrap();
        let m = mods::read(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        let mut app = bot_match(&m, 7);
        use super::super::*;
        let mut iso = hst_data::fixture::iso().unwrap();
        let chief = stats_of(&tparam(&mut iso, 7));
        {
            let g = app.world().resource::<Game>();
            let p = &g.players[1];
            assert_eq!((g.chars[1], p.stats.speed, p.stats.agility), (7, chief.speed / 4.0, chief.agility));
            assert_eq!(swing_pace(g, 1), 2.0);
            assert_eq!(swing_pace(g, 0), 1.0);
            let own = character_reach(&g.reach, &mut iso, 7, 1.0);
            assert!(g.reaches[1].reach > own.reach && g.reaches[1].smash_top == 2.0 * own.smash_top, "{:?} vs {own:?}", g.reaches[1]);
            assert_eq!(g.smash_heights[1], super::super::smash_heights(&mut iso, 7).map(|h| 2.0 * h));
            let head = |c: &character::CharacterData| c.skeleton.names.iter().position(|n| n == "Bip01Head").map(|h| hst_sim::pose::model(&c.skeleton, &c.skeleton.rest)[h][3][1]).unwrap();
            let pc07 = character::load_disc_with(&mut iso, 7, 0, &mut Assets::<Mesh>::default(), &mut Assets::<StandardMaterial>::default(), &mut Assets::<Image>::default(), &mut Assets::<SkinnedMeshInverseBindposes>::default(), None).unwrap();
            assert!((head(&g.data[1]) - 2.0 * head(&pc07)).abs() < 1e-4, "head {} vs pc07's {}", head(&g.data[1]), head(&pc07));
        }
        for frame in 0..60 * 60 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            if g.last_hitter == 1 && g.shots >= 1 {
                return eprintln!("{}: hit a ball at frame {frame}", m.id);
            }
        }
        panic!("{}: never hit a ball in a minute", m.id);
    }

    /// A bot singles match, Carol against mod `m` (set up as disc character `char`), after its first update.
    fn bot_match(m: &Mod, char: usize) -> App {
        use super::super::*;
        // every player a bot
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, char], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(MatchMod(vec![None, Some(m.clone())]));
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        app
    }

    /// Plays until a rally of three shots or more in one point with the mod (player 1) striking one of them.
    fn rally(app: &mut App, m: &Mod) {
        let (mut best, mut mod_hit) = (0, false);
        for frame in 0..60 * 120 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            best = best.max(g.shots);
            mod_hit |= g.last_hitter == 1 && g.shots >= 2;
            if best >= 3 && mod_hit {
                eprintln!("{}: frame {frame}, a {best}-shot rally, the mod hit", m.id);
                return;
            }
        }
        panic!("{}: longest rally {best} shots, mod returned a ball: {mod_hit}", m.id);
    }

    /// M1g: the chair umpires, packaged by the mods repo (`tools/rerig/hst_umpires.py` -> `mods/hst-mods`), are all
    /// [`mods::dropped`] (B381's four, Robot Tennis since 2026-10-19): none is on the custom roster (`mods::list` of
    /// `mods/`, as the select builds it; skipped without the umpire mods).
    #[test]
    fn umpires_stay_off_the_roster() {
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods"));
        if !root.join("hst-mods/hst_umpire_03_robottennis").is_dir() {
            return eprintln!("no umpire mods under mods/hst-mods, skipped");
        }
        let ump: Vec<String> = mods::list(root).into_iter().map(|m| m.id).filter(|id| id.starts_with("hst_umpire_")).collect();
        assert!(ump.is_empty(), "umpires on the roster: {ump:?}");
    }

    /// B181: a mod player speaks only its own takes, never its donor's. Helghast ships no voice: no bank at all, so
    /// nothing it does in the match (shouts, serve, reactions, emotes) sounds; Wendy a full set and Chika (an umpire)
    /// six takes: their banks hold no donor disc bank, so a program they lack is silent (skipped without the ISO or
    /// `mods/hst-mods`).
    #[test]
    fn mod_voices_never_fall_back_to_the_donor() {
        const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/");
        if !hst_data::fixture::has_iso() || !Path::new(ROOT).is_dir() {
            return eprintln!("no ISO or mods/hst-mods, skipped");
        }
        for id in ["getagrip_pc14_helghast", "getagrip_pc03_wendy", "hst_umpire_00_chika"] {
            let m = mods::read(&Path::new(ROOT).join(id)).unwrap();
            let app = bot_match(&m, m.donor);
            let banks = &app.world().resource::<crate::audio::VoiceBanks>().0;
            assert!(banks[0].is_some(), "{id}: the disc player keeps its voice");
            match (&banks[1], mods::voice(&m)) {
                (None, None) => {}
                (Some(b), Some(own)) => {
                    assert!(b.hd.is_empty() && b.bd.is_empty(), "{id}: donor disc bank attached");
                    assert_eq!(b.wav.keys().collect::<std::collections::BTreeSet<_>>(), own.wav.keys().collect(), "{id}");
                }
                (b, own) => panic!("{id}: bank {} but the mod's voice {}", b.is_some(), own.is_some()),
            }
        }
    }

    /// A packaged Get a Grip mod (`mods/hst-mods`, the mods repo's `out/mods`) in a match: the stats the game reads
    /// are its `params.override` cells (Get a Grip's stats ranked onto TParam), its AI row is `ai_row`.
    #[test]
    fn get_a_grip_stats_reach_the_match() {
        const GAG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/getagrip_pc03_wendy");
        if !hst_data::fixture::has_iso() || !Path::new(GAG).is_dir() {
            return eprintln!("no ISO or mods/hst-mods/getagrip_pc03_wendy, skipped");
        }
        let m = mods::read(Path::new(GAG)).unwrap();
        let o = |k: &str| m.overrides.iter().find(|(c, _)| c == k).unwrap_or_else(|| panic!("no {k} override")).1.parse::<i32>().unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![1, m.donor], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(MatchMod(vec![None, Some(m.clone())]));
        app.init_resource::<super::super::Pads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, super::super::setup);
        app.update();
        let mut iso = hst_data::fixture::iso().unwrap();
        let g = app.world().resource::<super::super::Game>();
        let p = &g.players[1];
        eprintln!("{}: speed {} agility {} stamina {}, power {:?}, aim {:?}, reach {}", m.id, p.stats.speed, p.stats.agility, p.stats.stamina, g.finish.power[1], g.aim_stats[1].con, g.reaches[1].reach);
        assert_eq!((p.stats.speed, p.stats.agility, p.stats.stamina), (o("SPE") as f32 / 10.0, o("Agili"), o("STA")));
        assert_eq!(g.finish.power[1], [o("Strk POW"), o("Voley POW")]);
        assert_eq!(g.aim_stats[1].con, [o("Strk CON"), o("Voley CON"), o("Serv CON")]);
        assert_eq!(g.reaches[1].reach, hst_sim::ps2::add(hst_sim::ps2::div(o("リーチ(cm)") as f32, 100.0), hst_sim::ps2::div(30.0, 100.0)));
        assert_eq!(p.ai, super::super::ai_params(&mut iso, m.ai_row, 0, false));
        assert_ne!(m.ai_row, m.donor, "Wendy's AI row should follow her play style, not her body donor");
    }

    /// Online (M2i): the mod with its looks and a stand-in from the same wire hand the game the same skeleton and
    /// motions, so a peer that declined the looks plays the same match.
    #[test]
    fn stand_in_simulates_like_the_mod() {
        if !hst_data::fixture::has_iso() || !hst_data::fixture::exists("context/mods/test_pc00") {
            return;
        }
        let w = crate::mods_online::Wire::of(&mods::read(&mod_dir()).unwrap(), 0).unwrap();
        let mut iso = hst_data::fixture::iso().unwrap();
        let mut one = |m: &Mod| {
            load(&mut iso, m, 0, &mut Assets::default(), &mut Assets::default(), &mut Assets::default(), &mut Assets::default())
        };
        let (real, stand) = (one(&crate::mods_online::for_match(&w, Some(&mod_dir()))), one(&crate::mods_online::for_match(&w, None)));
        assert_eq!((&real.skeleton.names, &real.skeleton.parent), (&stand.skeleton.names, &stand.skeleton.parent));
        assert!(real.skeleton.rest == stand.skeleton.rest && real.pelvis == stand.pelvis);
        let mut ids: Vec<_> = real.motions.keys().copied().collect();
        ids.sort();
        assert_eq!(ids, { let mut s: Vec<_> = stand.motions.keys().copied().collect(); s.sort(); s });
        for id in ids {
            let (a, b) = (&real.motions[&id], &stand.motions[&id]);
            for k in 0..8 {
                let t = a.length * k as f32 / 7.0;
                assert!(a.locals(&real.skeleton, t) == b.locals(&stand.skeleton, t), "motion {id:#x} at {t}");
            }
        }
        assert!(!stand.visual.is_empty() && real.joints.len() != stand.joints.len(), "the stand-in draws the donor's own rig");
    }
}

//! The game build a match plays by, from the disc's release (`hst_sim::build::Build`): a US disc plays the US rules,
//! the Japanese disc (SCPS-15113) its own earlier build's. Set once at match setup, after the players are loaded
//! (a mod character's own aim values included); the rest of the match asks `Game::build` where a rule differs.
//! Settings → Gameplay → Rules (M175, `crate::rules_choice`) can lay another release's disc over the main one, which
//! then reads as that release here too; online the lobby's host's disc decides (M171). A match at 50 Hz
//! (`hst_base::match_speed::current`, with `FIFTY_HZ_SIM` on) on the European disc plays its own retimed program.

use super::Game;
use hst_data::exe::Release;
use hst_data::iso::Iso;
use hst_sim::build::Build;

/// The build of the disc's release (US for anything that isn't the Japanese release).
pub(super) fn of(iso: &mut Iso) -> Build {
    let release = iso.read("SYSTEM.CNF").ok().and_then(|cnf| Release::of(&cnf).ok());
    build_at(release, hst_base::match_speed::sim_hz())
}

/// The build of `release` for a match at `hz`: the European program only at 50 Hz on the European disc.
fn build_at(release: Option<Release>, hz: u32) -> Build {
    match release {
        Some(Release::Japan) => Build::Japan,
        Some(Release::Europe) if hz == 50 => Build::Europe,
        _ => Build::Us,
    }
}

/// The match plays by `build`: the players' aim and serve data and the strike banners take it in.
pub(super) fn apply(g: &mut Game, build: Build) {
    g.build = build;
    for a in &mut g.aim_stats {
        a.build = build;
    }
    if !build.strong_toss_miss() {
        for d in g.serve_data.iter_mut() {
            d.miss = None;
        }
    }
    g.finish.no_counter = !build.counter();
    // M183: the timing the sim plays by (the European program's 50 Hz one): every relaunch keeps the flight's build
    g.flight.build = build;
    g.hit_flight.build = build;
    for p in &mut g.players {
        p.body.build = build;
        p.ai.build = build;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_japanese_release_changes_the_rules() {
        assert_eq!(build_at(Some(Release::Us), 60), Build::Us);
        assert_eq!(build_at(None, 60), Build::Us);
        assert_eq!(build_at(Some(Release::Japan), 60), Build::Japan);
        assert_eq!(build_at(Some(Release::Europe), 60), Build::Us);
    }

    #[test]
    fn the_european_disc_at_50_hz_plays_its_own_program() {
        assert_eq!(build_at(Some(Release::Europe), 50), Build::Europe);
        assert_eq!(build_at(Some(Release::Us), 50), Build::Us);
        assert_eq!(build_at(Some(Release::Japan), 50), Build::Japan);
    }
}

/// A bot match on the Japanese disc (`fixtures/discs/SCPS-15113.iso`; skipped without it even under
/// `HST_REQUIRE_DISC`, as it isn't the user's disc): it plays by the Japanese build, its CSVs read in the US layout, and the red landing marker shows from the
/// serve on.
#[cfg(test)]
mod japan {
    use super::super::*;
    use crate::{Args, GameSpace};
    use hst_sim::build::Build;

    #[test]
    fn a_bot_match_on_the_japanese_disc() {
        let iso = hst_data::fixture::path("fixtures/discs/SCPS-15113.iso");
        if !iso.exists() {
            // not the user's disc: skipped even under HST_REQUIRE_DISC
            return eprintln!("skipped: no fixtures/discs/SCPS-15113.iso");
        }
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: iso.to_string_lossy().into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 13], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, played_out, held_ball).chain());
        app.update();
        {
            let g = app.world().resource::<Game>();
            assert_eq!(g.build, Build::Japan);
            assert!(g.aim_stats.iter().all(|a| a.build == Build::Japan));
            assert!(g.serve_data.iter().all(|d| d.miss.is_none()));
            assert!(g.finish.no_counter);
            // TParam's Japanese cells land in the US columns: Eleanor's stamina (Japanese column 39, US 43)
            assert_eq!(g.players[0].stats.stamina, 40);
            // the serve tables are the rally table (its timing counts)
            let d = &g.serve_data[0];
            assert_eq!(d.strong_grades, d.weak_grades);
        }
        let (mut serves, mut serve_reds) = (0, 0);
        for _ in 0..60 * 40 {
            let before = app.world().resource::<Game>().shots;
            app.world_mut().run_schedule(FixedUpdate);
            let w = app.world();
            let g = w.resource::<Game>();
            if g.shots == 1 && before == 0 {
                serves += 1;
                serve_reds += w.resource::<effects::LandingMarks>().red_at.is_some() as i32;
                assert!(g.marks.red.is_none(), "the CPU's input stays the US one on the serve");
            }
        }
        assert!(serves > 0 && serve_reds == serves, "{serves} serves, {serve_reds} with the red marker");
    }
}

/// A bot match on the European disc alone (`fixtures/discs/SCES-54535.iso`; skipped without it even under
/// `HST_REQUIRE_DISC`, as it isn't the user's disc, M170): it plays by the US rules on the US shot records, its
/// trajectory tables read back regenerated for 60 Hz, and rallies play out point after point.
#[cfg(test)]
mod europe {
    use super::super::*;
    use crate::{Args, GameSpace};
    use hst_sim::build::Build;

    #[test]
    fn a_bot_match_on_the_european_disc_alone() {
        let iso = hst_data::fixture::path("fixtures/discs/SCES-54535.iso");
        if !iso.exists() {
            // not the user's disc: skipped even under HST_REQUIRE_DISC
            return eprintln!("skipped: no fixtures/discs/SCES-54535.iso");
        }
        let _turn = super::super::fifty_hz::EU_DISC.lock().unwrap_or_else(|e| e.into_inner());
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: iso.to_string_lossy().into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 13], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, played_out, held_ball).chain());
        app.update();
        assert_eq!(app.world().resource::<Game>().build, Build::Us);
        // the match's reads of a trajectory archive are the regenerated one, the disc's own bytes stay as they are
        let mut disc = hst_data::iso::Iso::open(&iso).unwrap();
        assert_ne!(disc.read("TRAJ/TRAJ00A.XB").unwrap(), disc.read_unlogged("TRAJ/TRAJ00A.XB").unwrap());
        let (mut serves, mut most) = (0, 0);
        for _ in 0..60 * 40 {
            let before = app.world().resource::<Game>().shots;
            app.world_mut().run_schedule(FixedUpdate);
            let shots = app.world().resource::<Game>().shots;
            serves += (shots == 1 && before == 0) as i32;
            most = most.max(shots);
        }
        assert!(serves >= 2 && most >= 3, "{serves} serves, longest rally {most} shots");
    }
}

/// M174: the European disc with the US disc lent to it (`hst_data::crossplay::lend`, a player with both discs) plays
/// the US game exactly: a bot match on it and one on the US disc run tick for tick to the same state hash (players,
/// ball, score, the match's random generators, motions). Skipped without `fixtures/discs/SCES-54535.iso` even under
/// `HST_REQUIRE_DISC` (not the user's disc). ~4 s.
#[cfg(test)]
mod europe_lent {
    use super::super::*;
    use crate::{Args, GameSpace};
    use hst_sim::build::Build;

    fn bot_match(iso: &std::path::Path) -> App {
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: iso.to_string_lossy().into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 3, stage: None, play: true, singles: true, chars: vec![0, 13], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, played_out, held_ball).chain());
        app.update();
        app
    }

    #[test]
    fn a_bot_match_on_the_european_disc_with_the_us_disc_lent_plays_the_us_match() {
        let eu = hst_data::fixture::path("fixtures/discs/SCES-54535.iso");
        if !eu.exists() {
            // not the user's disc: skipped even under HST_REQUIRE_DISC
            return eprintln!("skipped: no fixtures/discs/SCES-54535.iso");
        }
        let _turn = super::super::fifty_hz::EU_DISC.lock().unwrap_or_else(|e| e.into_inner());
        if !hst_data::fixture::has_iso() {
            return;
        }
        let us = hst_data::fixture::path(hst_data::fixture::ISO);
        // a link of its own, so the other tests read the European disc alone
        let link = std::env::temp_dir().join(format!("hst-lent-match-{}.iso", std::process::id()));
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(eu.canonicalize().unwrap(), &link).unwrap();
        struct Gone(std::path::PathBuf);
        impl Drop for Gone {
            fn drop(&mut self) {
                hst_data::crossplay::unlend(&self.0);
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _gone = Gone(link.clone());
        hst_data::crossplay::lend(&link, &us).unwrap();
        let (mut a, mut b) = (bot_match(&us), bot_match(&link));
        assert_eq!(b.world().resource::<Game>().build, Build::Us);
        let mut shots = 0;
        for tick in 0..60 * 30 {
            a.world_mut().run_schedule(FixedUpdate);
            b.world_mut().run_schedule(FixedUpdate);
            let (ha, hb) = (snapshot::state_hash(a.world_mut()), snapshot::state_hash(b.world_mut()));
            assert_eq!(ha, hb, "tick {tick}: the European disc's match drifted from the US one");
            shots = shots.max(a.world().resource::<Game>().shots);
        }
        assert!(shots >= 3, "longest rally {shots} shots");
    }
}

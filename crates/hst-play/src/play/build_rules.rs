//! The game build a match plays by, from the disc's release (`hst_sim::build::Build`): a US disc plays the US rules,
//! the Japanese disc (SCPS-15113) its own earlier build's. Set once at match setup, after the players are loaded
//! (a mod character's own aim values included); the rest of the match asks `Game::build` where a rule differs.
//! Settings → Gameplay → Rules (M175, `crate::rules_choice`) can lay another release's disc over the main one, which
//! then reads as that release here too; online the lobby's host's disc decides (M171).

use super::Game;
use hst_data::exe::Release;
use hst_data::iso::Iso;
use hst_sim::build::Build;

/// The build of the disc's release (US for anything that isn't the Japanese release).
pub(super) fn of(iso: &mut Iso) -> Build {
    let release = iso.read("SYSTEM.CNF").ok().and_then(|cnf| Release::of(&cnf).ok());
    build_of(release)
}

fn build_of(release: Option<Release>) -> Build {
    match release {
        Some(Release::Japan) => Build::Japan,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_japanese_release_changes_the_rules() {
        assert_eq!(build_of(Some(Release::Us)), Build::Us);
        assert_eq!(build_of(None), Build::Us);
        assert_eq!(build_of(Some(Release::Japan)), Build::Japan);
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

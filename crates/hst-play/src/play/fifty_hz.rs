//! M183: a match on the European disc at 50 Hz plays the European program's own timing (`hst_sim::build::Build::Europe`).
//! This module retimes what the match loads for it: the characters' motions play at the build's rate for each motion
//! (`Build::motion_rate`, `Build::clip_length`; swings and serve swings keep their frames), and the dummies' root paths
//! with their motion. The sim's own build switches are asked through `Game::build` where play calls them.

use hst_base::character::CharacterData;
use hst_sim::build::{Build, MOTION_TICKS};
use hst_sim::pose::Clip;
use std::borrow::Cow;

/// The motions `d` plays, as `build` plays them: each clip at its motion's rate (a player's motions 0..47 by their
/// number, everything after them as no player motion), the visual takes with them, and every root path at the rate
/// of the motion it carries (the team reactions' paths as theirs). Nothing changes at 60 Hz.
pub(super) fn retime(d: &mut CharacterData, build: Build) {
    if !build.fifty() {
        return;
    }
    for (&k, clip) in d.motions.iter_mut().chain(d.visual.iter_mut()) {
        *clip = at_rate(clip, build, player_motion(k));
    }
    for (&k, path) in d.paths.iter_mut() {
        *path = path.at_rate(build.motion_rate(path.ticks_per_frame() as i32, player_motion(k)) as f32);
    }
    // the serve stance's ball track (the dummies' ball one: rescaled)
    if let Some(p) = d.stance_ball.as_mut() {
        *p = p.at_rate(build.motion_rate(p.ticks_per_frame() as i32, None) as f32);
    }
}

/// A motion number as `Build::motion_rate` reads it: a player's own 0..47, `None` for the team reactions and poses after.
fn player_motion(k: usize) -> Option<usize> {
    (k < 0x30).then_some(k)
}

/// `clip` (loaded at its file's rate) played at `build`'s rate for motion `index`: what `Clip::for_build` makes of
/// the same file.
pub(super) fn at_rate(clip: &Clip, build: Build, index: Option<usize>) -> Clip {
    let ticks = clip.ticks_per_frame as i32;
    let rate = build.motion_rate(ticks, index) as f32;
    if rate == clip.ticks_per_frame {
        return clip.clone();
    }
    Clip { ticks_per_frame: rate, length: build.clip_length(end_tick(clip), rate), tracks: clip.tracks.clone() }
}

/// The clip's last key tick, a whole number: its length is that over its ticks per frame, chopped, so the product
/// comes back within far less than half a tick.
fn end_tick(clip: &Clip) -> f32 {
    (clip.length as f64 * clip.ticks_per_frame as f64).round() as f32
}

/// `clip` at the motion files' own rate (80 ticks a frame): the toss's hand is sampled on the frame the US program
/// releases the ball at, the same tick the 50 Hz program releases it (`Build::toss_release`).
pub(super) fn file_rate(clip: &Clip) -> Cow<'_, Clip> {
    let rate = MOTION_TICKS as f32;
    if clip.ticks_per_frame == rate {
        return Cow::Borrowed(clip);
    }
    let end = end_tick(clip);
    Cow::Owned(Clip { ticks_per_frame: rate, length: Build::Us.clip_length(end, rate), tracks: clip.tracks.clone() })
}

/// The ticks a voice line of `frames` 60 Hz frames lasts at `build`'s rate (`frames × 50 / 60` at 50 Hz, rounded
/// down): the audio's length in real time doesn't change.
pub(super) fn voice_ticks(frames: u32, build: Build) -> u32 {
    frames * build.hz() / 60
}

/// The tests that play a match on the European disc take turns: the 50 Hz one sets the process's sim rate and the
/// disc's own-rate reads (`hst_base::match_speed::test_sim_hz`), which the others must not see.
#[cfg(test)]
pub(super) static EU_DISC: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A bot match on the European disc at 50 Hz (`fixtures/discs/SCES-54535.iso`; skipped without it even under
/// `HST_REQUIRE_DISC`, as it isn't the user's disc), the sim's 50 Hz timing on before `FIFTY_HZ_SIM`: it plays the
/// European program (`Build::Europe` everywhere play asks), on the disc's own trajectory tables, with every motion at
/// the European program's rate, and rallies and the change of ends play out on its timing.
#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{Args, GameSpace};
    use hst_sim::build::Build;

    const EU: &str = "fixtures/discs/SCES-54535.iso";

    struct Sixty;
    impl Drop for Sixty {
        fn drop(&mut self) {
            hst_base::match_speed::test_sim_hz(0);
        }
    }

    #[test]
    fn a_bot_match_at_50_hz_plays_the_european_program() {
        let iso = hst_data::fixture::path(EU);
        if !iso.exists() {
            // not the user's disc: skipped even under HST_REQUIRE_DISC
            return eprintln!("skipped: no {EU}");
        }
        let _turn = super::EU_DISC.lock().unwrap_or_else(|e| e.into_inner());
        hst_base::match_speed::test_sim_hz(50);
        let _sixty = Sixty;
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: iso.to_string_lossy().into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 0, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, played_out, held_ball).chain());
        app.update();
        {
            let g = app.world().resource::<Game>();
            assert_eq!(g.build, Build::Europe);
            assert!(g.flight.build == Build::Europe && g.hit_flight.build == Build::Europe);
            assert!(g.players.iter().all(|p| p.body.build == Build::Europe && p.ai.build == Build::Europe));
            // every player motion as the European program loads it (`Clip::for_build` from the disc's own file)
            let mut disc = hst_data::iso::Iso::open(&iso).unwrap();
            let anims = disc.read("PCANI/PC00ANI.XB").unwrap();
            let arc = hst_data::xb::Archive::parse(&anims).unwrap();
            let (sk, mut checked, mut kept) = (&g.data[0].skeleton, 0, 0);
            for id in 0..0x30 {
                let (Some(stem), Some(clip)) = (hst_data::ani::motion_name(id, 0), g.data[0].motions.get(&id)) else { continue };
                let stem = format!("{}.ani2", stem.to_ascii_lowercase());
                let Some(e) = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&stem)) else { continue };
                let a = hst_data::ani::parse(&arc.read(e).unwrap()).unwrap();
                let want = hst_sim::pose::Clip::for_build(sk, &a, Build::Europe, Some(id));
                assert_eq!((clip.ticks_per_frame, clip.length.to_bits()), (want.ticks_per_frame, want.length.to_bits()), "motion {id}");
                checked += 1;
                kept += (clip.ticks_per_frame == 80.0) as i32;
            }
            assert!(checked > 30 && kept > 0 && kept < checked, "{checked} motions, {kept} at the US rate");
            // the match's trajectory archives are the disc's own (50 Hz) tables, not regenerated
            assert_eq!(disc.read("TRAJ/TRAJ00A.XB").unwrap(), disc.read_disc("TRAJ/TRAJ00A.XB").unwrap());
        }
        let (mut serves, mut most, mut change_ends) = (0, 0, Vec::new());
        let mut tosses = 0;
        for _ in 0..50 * 150 {
            let before = app.world().resource::<Game>().clone_phase();
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            serves += (g.shots == 1 && before.1 == 0) as i32;
            most = most.max(g.shots);
            if let (Phase::ChangeEnds(n), false) = (g.phase.clone(), matches!(before.0, Phase::ChangeEnds(_))) {
                change_ends.push(n);
            }
            tosses += (g.serving.tossed && !before.2) as i32;
        }
        assert!(serves >= 2 && most >= 3 && tosses >= serves, "{serves} serves, {tosses} tosses, longest rally {most} shots");
        assert!(!change_ends.is_empty() && change_ends.iter().all(|&n| n == 66), "change ends at {change_ends:?}");
    }

    /// The recorded European bot match (`fixtures/m169/eu-bot-match.bin`, M177): its phases from the first serve
    /// set-up, each as (phase, vsyncs, the players' positions as it starts), the game's phase numbers mapped to
    /// play's (2 serve set-up, 3 rally, 4 point over, 1 change of ends).
    fn recorded_phases(data: &[u8]) -> Vec<(u8, u32, [[f32; 3]; 4])> {
        let frames = hst_sim::replay::frames_live(data);
        let first = frames.iter().position(|f| f.gm()[0x55] == 2).unwrap();
        let mut out: Vec<(u8, u32, [[f32; 3]; 4])> = Vec::new();
        let mut start = frames[first].vsync();
        for &f in &frames[first..] {
            let ph = f.gm()[0x55];
            if out.last().is_none_or(|l| l.0 != ph) {
                if let Some(l) = out.last_mut() {
                    l.1 = f.vsync() - start;
                }
                start = f.vsync();
                out.push((ph, 0, std::array::from_fn(|i| f.player_pos(i))));
            }
        }
        out.pop();
        out
    }

    /// Probe, not a proof yet (M183a1): the European bot match (doubles, characters 0/1/1/0, court 6, 1 set of 4 games)
    /// in play from the original's match seed (the 875 477th `rand()` from boot, as the recording's RAM holds it),
    /// listed phase by phase beside the recording. Asserts nothing: as of M183a the first serve set-up already differs
    /// (135 recorded vsyncs, 215 in play). Run by hand while chasing that:
    /// `tools/check.sh -p hst-play the_european_bot_match_plays_as_recorded -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn the_european_bot_match_plays_as_recorded() {
        let iso = hst_data::fixture::path(EU);
        let Ok(data) = std::fs::read(hst_data::fixture::path("fixtures/m169/eu-bot-match.bin")) else { return eprintln!("skipped") };
        if !iso.exists() {
            return eprintln!("skipped: no {EU}");
        }
        let _turn = super::EU_DISC.lock().unwrap_or_else(|e| e.into_inner());
        hst_base::match_speed::test_sim_hz(50);
        let _sixty = Sixty;
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        let args = Args { iso: iso.to_string_lossy().into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 6, stage: None, play: true, singles: false, chars: vec![0, 1, 1, 0], outfits: vec![], viewer: None, sound: None, music: false, umpire: 0, sets: 1, games: 4, pads: None };
        // the match seed as main.rs draws it: the menus' `rand()` calls, then the weather schedule and the setup's draws
        let seed = hst_sim::rng::Seed::after(875_476);
        let mut disc = hst_data::iso::Iso::open(&iso).unwrap();
        let (cnf, bin) = (disc.read("SYSTEM.CNF").unwrap(), disc.read("ZZBIN/GAME.BIN").unwrap());
        let exe = hst_data::exe::Game::new(&cnf, &bin).unwrap();
        let o = exe.weather_odds(6);
        let odds = hst_sim::weather::Odds { cloudy: o[0], cloudy_len: [o[1], o[2]], rain: o[3], rain_len: [o[4], o[5]], no_border: o[7] != 0, heavy: o[8] != 0 };
        let ((directions, speed), (chance, kind)) = (exe.wind(6), exe.gusts(6));
        let wind = hst_sim::weather::Wind { chance, kind, directions, speed };
        let mut mt = hst_sim::weather::Mt::new(seed.shared);
        let schedule = hst_sim::weather::schedule(&odds, &wind, 4, 1, 4, || mt.next());
        let mut rngs = hst_sim::rng::Rngs::new(seed.rand, mt);
        let (effects, _) = rngs.setup_rand(0);
        set_effects_seed(effects);
        app.insert_resource(MatchRng(rngs));
        app.insert_resource(crate::weather::Weather { schedule, game: 0, fixed: None });
        app.insert_resource(args);
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let want = recorded_phases(&data);
        let code = |p: &Phase| match p {
            Phase::Serve => 2,
            Phase::Rally => 3,
            Phase::Post => 4,
            Phase::ChangeEnds(_) => 1,
        };
        let mut got: Vec<(u8, u32, [[f32; 3]; 4])> = Vec::new();
        let pos = |g: &Game| std::array::from_fn(|i| g.players[i].body.pos);
        {
            let g = app.world().resource::<Game>();
            got.push((code(&g.phase), 0, pos(g)));
        }
        let total: u32 = want.iter().map(|w| w.1).sum();
        for _ in 0..total + 10 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            let c = code(&g.phase);
            let l = got.last_mut().unwrap();
            l.1 += 1;
            if l.0 != c {
                l.1 -= 1;
                got.push((c, 1, pos(g)));
            }
        }
        for (k, (w, g)) in want.iter().zip(&got).enumerate().take(60) {
            eprintln!("{k:3} want {} {:4} {:?} | got {} {:4} {:?}", w.0, w.1, w.2[0], g.0, g.1, g.2[0]);
        }
        let same = want.iter().zip(&got).take_while(|(w, g)| w.0 == g.0 && w.1 == g.1).count();
        eprintln!("{same} of {} phases match", want.len());
    }

    impl Game {
        fn clone_phase(&self) -> (Phase, u32, bool) {
            (self.phase.clone(), self.shots as u32, self.serving.tossed)
        }
    }
}

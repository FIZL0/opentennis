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
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, played_out, held_ball, crate::play::bodyhit::pose, crate::play::doubles_ai::scratch::after_pose).chain());
        app.init_resource::<crate::play::bodyhit::Posed>().init_resource::<crate::play::bodyhit::Bones>();
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

    /// One tick of a bot match: the phase in the recording's numbers (2 serve set-up, 3 rally, 4 point over, 1 change
    /// of ends), the players' positions and the ball's (game space, Y down).
    type Tick = (u8, [[f32; 3]; 4], [f32; 3]);

    /// A recorded bot match (`frames_live`) tick by tick from its first serve set-up, one entry a game tick (the
    /// phase's tick count, gm+0x58), as play counts them: the game stalls a few vsyncs while it loads (B723: three
    /// in the point-over phase as the scoreboard's pause ends, three as it leaves for the next serve) with every
    /// object frozen, and those vsyncs aren't ticks. A tick that ran unsampled repeats the sample before it, or at a
    /// phase's start (the next serve's first tick runs in the stall) the phase's first sample.
    fn recorded_ticks(data: &[u8]) -> Vec<Tick> {
        let frames = hst_sim::replay::frames_live(data);
        let first = frames.iter().position(|f| f.gm()[0x55] == 2).unwrap();
        let tick = |f: &hst_sim::replay::Frame| u32::from_le_bytes(f.gm()[0x58..0x5c].try_into().unwrap());
        let sample = |f: &hst_sim::replay::Frame| -> Tick {
            let b = f.live_ball();
            // the live position (+0x120 is the copy each flight step saves first: a tick behind in flight)
            let ball = [0, 4, 8].map(|o| f32::from_le_bytes(b[0xe0 + o..0xe4 + o].try_into().unwrap()));
            (f.gm()[0x55], std::array::from_fn(|i| f.player_pos(i)), ball)
        };
        let mut out = vec![sample(&frames[first])];
        for w in frames[first..].windows(2) {
            let (a, b) = (sample(&w[0]), sample(&w[1]));
            if a.0 == b.0 {
                let n = tick(&w[1]).saturating_sub(tick(&w[0])) as usize;
                if n > 0 {
                    out.extend(std::iter::repeat_n(a, n - 1));
                    out.push(b);
                }
            } else {
                out.extend(std::iter::repeat_n(b, tick(&w[1]).max(1) as usize));
            }
        }
        out
    }

    /// `ticks` as phases: (phase, ticks, the players' positions as it starts); the last, unfinished one dropped.
    fn phases(ticks: &[Tick]) -> Vec<(u8, u32, [[f32; 3]; 4])> {
        let mut out: Vec<(u8, u32, [[f32; 3]; 4])> = Vec::new();
        for t in ticks {
            match out.last_mut() {
                Some(l) if l.0 == t.0 => l.1 += 1,
                _ => out.push((t.0, 1, t.1)),
            }
        }
        out.pop();
        out
    }

    /// A bot match in play (all CPU) from the original's match seed as main.rs sets it up (the weather schedule, the
    /// setup's draws), `n` ticks of it from the setup, at the sim rate already set, with the match's generators after
    /// each of the first `keep` ticks. `toss_ai` AI draws stand in at
    /// every toss for the ones play doesn't make yet (the game's AI generator draws 60 there; M183a1's journal), so
    /// what parts after them shows.
    fn played_ticks_rng(args: Args, seed: hst_sim::rng::Seed, n: usize, keep: usize, toss_ai: usize) -> (Vec<Tick>, Vec<hst_sim::rng::Rngs>) {
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        let mut disc = hst_data::iso::Iso::open(&args.iso).unwrap();
        let (cnf, bin) = (disc.read("SYSTEM.CNF").unwrap(), disc.read("ZZBIN/GAME.BIN").unwrap());
        let exe = hst_data::exe::Game::new(&cnf, &bin).unwrap();
        let o = exe.weather_odds(args.court as u32);
        let odds = hst_sim::weather::Odds { cloudy: o[0], cloudy_len: [o[1], o[2]], rain: o[3], rain_len: [o[4], o[5]], no_border: o[7] != 0, heavy: o[8] != 0 };
        let ((directions, speed), (chance, kind)) = (exe.wind(args.court as u32), exe.gusts(args.court as u32));
        let wind = hst_sim::weather::Wind { chance, kind, directions, speed };
        let mut mt = hst_sim::weather::Mt::new(seed.shared);
        let players = if args.singles { 2 } else { 4 };
        let schedule = hst_sim::weather::schedule(&odds, &wind, args.games, args.sets, players, || mt.next());
        let mut rngs = hst_sim::rng::Rngs::new(seed.rand, mt);
        let (effects, _) = rngs.setup_rand(0);
        set_effects_seed(effects);
        app.insert_resource(MatchRng(rngs));
        app.insert_resource(crate::weather::Weather { schedule, game: 0, fixed: None });
        app.insert_resource(args);
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, flare_tick, effects::tick, react_first, control, reaction_roots, simulate, start_effects, motions, character::tick, played_out, held_ball, crate::play::bodyhit::pose, crate::play::doubles_ai::scratch::after_pose).chain());
        app.init_resource::<ReactionRoots>();
        app.init_resource::<crate::play::bodyhit::Posed>().init_resource::<crate::play::bodyhit::Bones>();
        app.update();
        let code = |p: &Phase| match p {
            Phase::Serve => 2,
            Phase::Rally => 3,
            Phase::Post => 4,
            Phase::ChangeEnds(_) => 1,
        };
        let tick = |g: &Game| (code(&g.phase), std::array::from_fn(|i| g.players[i].pos), g.flight.ball.pos);
        let mut out = vec![tick(app.world().resource::<Game>())];
        let mut rngs = vec![app.world().resource::<Game>().rng.clone()];
        for k in 1..n {
            let tossed = app.world().resource::<Game>().serving.tossed;
            app.world_mut().run_schedule(FixedUpdate);
            let mut g = app.world_mut().resource_mut::<Game>();
            if g.serving.tossed && !tossed {
                (0..toss_ai).for_each(|_| _ = g.rng.ai.next());
            }
            out.push(tick(app.world().resource::<Game>()));
            if k < keep {
                rngs.push(app.world().resource::<Game>().rng.clone());
            }
        }
        (out, rngs)
    }

    /// The two side by side: the first phases, how many match in kind and length, and the first tick a player stands
    /// elsewhere. Returns the phases that match.
    fn compare(name: &str, want: &[Tick], got: &[Tick]) -> usize {
        let (w, g) = (phases(want), phases(got));
        for (k, (w, g)) in w.iter().zip(&g).enumerate().take(30) {
            eprintln!("{name} {k:3} want {} {:4} {:?} | got {} {:4} {:?}", w.0, w.1, w.2[0], g.0, g.1, g.2[0]);
        }
        let same = w.iter().zip(&g).take_while(|(w, g)| w.0 == g.0 && w.1 == g.1).count();
        eprintln!("{name}: {same} of {} phases match", w.len());
        // play's tick k + shift beside the recording's tick k: where the players, then the ball, first part
        for shift in 0..3 {
            let got = &got[shift..];
            let players = want.iter().zip(got).position(|(w, g)| (w.0, w.1) != (g.0, g.1));
            let ball = want.iter().zip(got).position(|(w, g)| w.2 != g.2);
            eprintln!("{name} shift {shift}: players part at {players:?}, the ball at {ball:?}");
            for t in [players, ball].into_iter().flatten() {
                for k in t.saturating_sub(1)..(t + 3).min(want.len()) {
                    eprintln!("{name}   tick {k}: want {:?}\n{name}             got  {:?}", want[k], got[k]);
                }
            }
        }
        same
    }

    /// The first serve of `got` (play) beside `want` (the recording) from its set-up to the contact, as play's
    /// ticks line up with the game's: play's setup state is the game's last frame before the set-up, so play's tick
    /// k + 1 is the recording's tick k; play steps a launched ball in the tick it leaves the hand or the racket (the
    /// game's ball update, first in its next frame), so from the toss on play's ball is a tick ahead and play's tick
    /// k has the recording's ball of tick k. Panics where they part: every player stands where the recording has
    /// them (on the ground) through the contact; the held ball follows the hand bit for bit from `held` until the
    /// release; the toss flies the recorded curve bit for bit from the tick after the release to the contact; the
    /// served ball flies the recorded curve bit for bit from the tick after the contact; the match turns to rally the
    /// tick after the swing's `press`.
    fn serve_to_contact(name: &str, want: &[Tick], got: &[Tick], held: usize, (release, press, contact): (usize, usize, usize)) {
        // on the ground: the overhead serve's jump isn't applied in play (`serve_ik_start`)
        let ground = |t: &Tick| t.1.map(|p| [p[0], p[2]]);
        for k in 0..=contact {
            assert_eq!(ground(&want[k]), ground(&got[k + 1]), "{name} tick {k}: the players");
        }
        for k in held..release {
            assert_eq!(want[k].2.map(f32::to_bits), got[k + 1].2.map(f32::to_bits), "{name} tick {k}: the held ball");
        }
        // the release: the recording's ball still at the hand point, play's (a tick ahead) on its first step
        assert_ne!(want[release].2, want[release + 1].2, "{name}: the toss leaves at {release}");
        for k in release + 1..=contact {
            assert_eq!(want[k].2.map(f32::to_bits), got[k].2.map(f32::to_bits), "{name} tick {k}: the toss");
        }
        let toss_on = |t: &Tick, prev: &Tick| (t.2[0] - prev.2[0]).abs() < 0.01;
        assert!(!toss_on(&want[contact + 1], &want[contact]) && !toss_on(&got[contact + 1], &got[contact]), "{name}: struck at {contact}");
        for k in contact + 1..contact + 4 {
            assert_eq!(want[k].2.map(f32::to_bits), got[k].2.map(f32::to_bits), "{name} tick {k}: the served ball");
        }
        // the phase: rally from the tick after the swing press (play's a tick ahead, as its ball is)
        let rally = want.iter().position(|t| t.0 == 3).unwrap();
        assert_eq!(rally, press + 1, "{name}: rally from the tick after the press");
        for k in 0..=contact {
            assert_eq!(want[k].0, got[k].0, "{name} tick {k}: the phase");
        }
    }

    /// After `serve_to_contact`: the served ball flies the recorded curve bit for bit through its bounce in the
    /// receivers' court up to the return's contact `ret` (play level with the recording, as from the toss on), and
    /// every player stands where the recording has them through `stand` (the EU match: through the returner's slide
    /// into the return, the contact IK's body step, to the tick before the return reaches the server's side).
    fn serve_to_return(name: &str, want: &[Tick], got: &[Tick], contact: usize, stand: usize, ret: usize) {
        for k in contact + 1..=ret {
            assert_eq!(want[k].2.map(f32::to_bits), got[k].2.map(f32::to_bits), "{name} tick {k}: the served ball");
        }
        assert!(want[ret + 1].2[2].abs() < want[ret].2[2].abs(), "{name}: returned at {ret}");
        let ground = |t: &Tick| t.1.map(|p| [p[0].to_bits(), p[2].to_bits()]);
        for k in contact..=stand {
            assert_eq!(ground(&want[k]), ground(&got[k + 1]), "{name} tick {k}: the players");
        }
    }

    /// After `serve_to_return`: the return flies the recorded curve bit for bit from the tick after its contact `ret`
    /// through `to` (play level with the recording), so its aim and launch are the game's: the aim reads the
    /// computer player's stick as the pad state it wrote, in steps of 1/127.
    fn return_flight(name: &str, want: &[Tick], got: &[Tick], ret: usize, to: usize) {
        for k in ret + 1..=to {
            assert_eq!(want[k].2.map(f32::to_bits), got[k].2.map(f32::to_bits), "{name} tick {k}: the returned ball");
        }
    }

    /// Probe: from the serve's `contact` on, the first tick a player (on the ground, play a tick behind) or the ball
    /// (play level) leaves the recording, and how far, through the first point.
    fn first_point_part(name: &str, want: &[Tick], got: &[Tick], contact: usize) {
        let end = want.iter().skip(contact).position(|t| t.0 != 3).map_or(want.len(), |n| n + contact);
        let ground = |t: &Tick| t.1.map(|p| [p[0], p[2]]);
        let player = (contact..end).find(|&k| ground(&want[k]) != ground(&got[k + 1]));
        let ball = (contact + 1..end).find(|&k| want[k].2 != got[k].2);
        eprintln!("{name} point 1 (rally to {end}): a player parts at {player:?}, the ball at {ball:?}");
        // `HST_M183A1_PLAYER=i,from,to`: player i's ground position tick by tick, recorded beside played
        if let Some(v) = std::env::var("HST_M183A1_PLAYER").ok().map(|v| v.split(',').filter_map(|x| x.parse::<usize>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 3) {
            for k in v[1]..v[2].min(want.len() - 1) {
                let (w, g) = (want[k].1[v[0]], got[k + 1].1[v[0]]);
                eprintln!("{name} player {} tick {k}: want [{}, {}, {}] got [{}, {}, {}]", v[0], w[0], w[1], w[2], g[0], g[1], g[2]);
            }
        }
        for k in player.into_iter().chain(ball).flat_map(|t| t.saturating_sub(1)..(t + 4).min(end)) {
            let off = (0..4).map(|i| (0..3).map(|a| (want[k].1[i][a] - got[k + 1].1[i][a]).abs()).fold(0f32, f32::max)).collect::<Vec<_>>();
            let b = (0..3).map(|a| (want[k].2[a] - got[k].2[a]).abs()).fold(0f32, f32::max);
            eprintln!("{name}   tick {k}: players off {off:?}, ball off {b} (want {:?} got {:?})", want[k].2, got[k].2);
        }
    }

    /// Probe (`HST_M183A1_FROM=k`): from recorded tick `k` on, the first tick the phase or a player (on the ground,
    /// play a tick behind) or the ball (play level) leaves the recording, with a few ticks around it.
    fn part_from(name: &str, want: &[Tick], got: &[Tick]) {
        let Some(from) = std::env::var("HST_M183A1_FROM").ok().and_then(|v| v.parse::<usize>().ok()) else { return };
        let end = want.len().min(got.len() - 1);
        let ground = |t: &Tick| t.1.map(|p| [p[0], p[2]]);
        let phase = (from..end).find(|&k| want[k].0 != got[k + 1].0);
        let player = (from..end).find(|&k| ground(&want[k]) != ground(&got[k + 1]));
        let ball = (from..end).find(|&k| want[k].2 != got[k].2);
        eprintln!("{name} from {from}: the phase parts at {phase:?}, a player at {player:?}, the ball at {ball:?}");
        for k in ball.into_iter().chain(player).flat_map(|t| t.saturating_sub(1)..(t + 3).min(end)) {
            eprintln!("{name}   tick {k}: want {} {:?} {:?}\n{name}             got  {} {:?} {:?} (ball {:?})", want[k].0, ground(&want[k]), want[k].2, got[k + 1].0, ground(&got[k + 1]), got[k + 1].2, got[k].2);
        }
    }

    /// The European bot match (doubles, characters 0/1/1/0, court 6, 1 set of 4 games; M177's recording) in play.
    fn european(n: Option<usize>, toss_ai: usize) -> Option<(Vec<Tick>, Vec<Tick>)> {
        european_rngs(n, toss_ai, 0).map(|(w, g, _)| (w, g))
    }

    /// [`european`] with the match's generators after each of the first `keep` ticks.
    fn european_rngs(n: Option<usize>, toss_ai: usize, keep: usize) -> Option<(Vec<Tick>, Vec<Tick>, Vec<hst_sim::rng::Rngs>)> {
        let iso = hst_data::fixture::path(EU);
        let Ok(data) = std::fs::read(hst_data::fixture::path("fixtures/m169/eu-bot-match.bin")) else {
            eprintln!("skipped: no eu-bot-match.bin");
            return None;
        };
        if !iso.exists() {
            eprintln!("skipped: no {EU}");
            return None;
        }
        let _turn = super::EU_DISC.lock().unwrap_or_else(|e| e.into_inner());
        hst_base::match_speed::test_sim_hz(50);
        let _sixty = Sixty;
        let args = Args { iso: iso.to_string_lossy().into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 6, stage: Some(6), play: true, singles: false, chars: vec![0, 1, 1, 0], outfits: vec![], viewer: None, sound: None, music: false, umpire: 0, sets: 1, games: 4, pads: None };
        let want = recorded_ticks(&data);
        // the match seed: the 81 875th `rand()` output from boot (`eu_match_seed_search`)
        let (got, rngs) = played_ticks_rng(args, hst_sim::rng::Seed::after(81_874), n.unwrap_or(want.len() + 10), keep, toss_ai);
        Some((want, got, rngs))
    }

    /// The European match's first serve in play from the original's seed, beside the recording (M184's tick table):
    /// the server walks to the same spot on the same ticks, tosses at 76, the ball leaves the hand at 114 (the posed
    /// toss frame plus its speed reaching 37.5: 38 ticks after the press), the AI presses at 134 and strikes at 141;
    /// every player stands where the recording has them and the ball follows the hand and the toss curve bit for bit
    /// to the contact.
    #[test]
    fn the_european_first_serve_set_up_plays_as_recorded() {
        let Some((want, got)) = european(Some(450), 0) else { return };
        serve_to_contact("eu", &want, &got, 8, (114, 134, 141));
        // every player through the server's partner's six steps after the return (250-255), its stand after them
        // (the walk back's singles-column roll, B719) and its partner's next contact (its frame stamp, M183a1), to
        // the receiver's whiff and the rally's end (its miss motion's 31-frame release, M183a1)
        serve_to_return("eu", &want, &got, 141, 437, 230);
        // the whole rally after the return: the server's return back, the whiff, the ball to the end of the rally
        return_flight("eu", &want, &got, 230, 437);
    }

    /// B723: the European match's first point ends as the game ends it: the rally phase runs two ticks past the ball's
    /// deciding bounce (the game's check reads the ball a tick late, and the point-over phase starts the tick after
    /// it), the point-over phase runs its 134 ticks and the next serve is set up on the game's tick (the recording
    /// counted in game ticks: its loading stalls freeze everything and aren't ticks). Players stand where the
    /// recording has them through the scoreboard's pause (463: they part at the reaction after it), the ball flies
    /// as recorded to 470 (it meets the stage there in the game, not in play).
    #[test]
    fn the_european_first_point_ends_as_recorded() {
        let Some((want, got)) = european(Some(600), 0) else { return };
        let (w, g) = (phases(&want), phases(&got[..600]));
        let kinds = |p: &[(u8, u32, [[f32; 3]; 4])]| p.iter().take(3).map(|p| (p.0, p.1)).collect::<Vec<_>>();
        assert_eq!(kinds(&w), [(2, 135), (3, 303), (4, 134)], "the recording's phases");
        assert_eq!(kinds(&g), kinds(&w), "the phases in play");
        let next = |t: &[Tick]| (438..t.len()).find(|&k| t[k].0 == 2);
        assert_eq!(next(&got), next(&want), "the next serve's set-up");
        let ground = |t: &Tick| t.1.map(|p| [p[0].to_bits(), p[2].to_bits()]);
        for k in 437..=463 {
            assert_eq!(ground(&want[k]), ground(&got[k + 1]), "eu tick {k}: the players");
        }
        for k in (437..=470).filter(|&k| k != 464) {
            assert_eq!(want[k].2.map(f32::to_bits), got[k].2.map(f32::to_bits), "eu tick {k}: the ball");
        }
    }

    /// Probe, not a proof yet (M183a1): the whole European bot match in play from the original's match seed, beside the
    /// recording phase by phase and tick by tick. Asserts only that the first point's phases match (B723). Run by hand:
    /// `tools/check.sh -p hst-play bot_match_plays_as_recorded -- --ignored --nocapture` (`HST_M183A1_TOSS_AI=n`: n
    /// AI draws at each toss, `HST_M183A1_DUMP`: the first 240 ticks of the server and the ball, `=from,to` others).
    #[test]
    #[ignore]
    fn the_european_bot_match_plays_as_recorded() {
        let toss_ai = std::env::var("HST_M183A1_TOSS_AI").ok().and_then(|n| n.parse().ok()).unwrap_or(0);
        let Some((want, got)) = european(None, toss_ai) else { return };
        assert!(compare("eu", &want, &got) >= 3, "the first point's phases (B723)");
        first_point_part("eu", &want, &got, 141);
        part_from("eu", &want, &got);
        // B723: after the point's rally, the first tick a player (on the ground, play a tick behind) or the ball parts
        let ground = |t: &Tick| t.1.map(|p| [p[0], p[2]]);
        let player = (437..want.len() - 1).find(|&k| ground(&want[k]) != ground(&got[k + 1]));
        let ball = (437..want.len() - 1).find(|&k| want[k].2 != got[k].2);
        eprintln!("eu after point 1: players part at {player:?}, the ball at {ball:?}");
        // past the stall's unsampled tick (recorded 464 repeats 463)
        let ball2 = (465..want.len() - 1).find(|&k| want[k].2 != got[k].2);
        eprintln!("eu after the stall: the ball parts at {ball2:?}");
        for k in ball.into_iter().chain(ball2).flat_map(|p| p.saturating_sub(1)..p + 2) {
            eprintln!("eu   ball {k}: want {:?} got {:?}", want[k].2, got[k].2);
        }
        for k in player.into_iter().flat_map(|p| p.saturating_sub(1)..p + 2) {
            eprintln!("eu   tick {k}: want {:?} {:?}\neu             got  {:?} {:?}", want[k].0, want[k].1, got[k + 1].0, got[k + 1].1);
        }
        if let Some(v) = std::env::var_os("HST_M183A1_DUMP") {
            // `from,to`: another span of recorded ticks
            let span: Vec<usize> = v.to_string_lossy().split(',').filter_map(|x| x.parse().ok()).collect();
            let (from, to) = if span.len() == 2 { (span[0], span[1]) } else { (0, 240) };
            for k in from..to.min(want.len()) {
                eprintln!("eu dump {k}: want {} {:?} {:?} | got {} {:?} {:?}", want[k].0, want[k].1[0], want[k].2, got[k + 1].0, got[k + 1].1[0], got[k + 1].2);
            }
        }
    }

    /// M183a1: the European match's generators in play beside the ones recorded from `eu-bot-start.p2s`
    /// (`fixtures/m169/eu-rng.bin`, `eu_rng_rec.py` in the journal: vsync, then the shared, AI and court generators):
    /// for each played tick, the recorded tick (from the first serve set-up) each generator equals, and the AI draws a
    /// tick around the return (`--nocapture`; `HST_M183A1_DRAWS=from,to`: another span); asserts the AI generator
    /// follows the recording through the first point's rally and into its point-over phase (recorded 463).
    #[test]
    fn the_european_generators_play_as_recorded() {
        use hst_sim::rng::Mt;
        let Ok(d) = std::fs::read(hst_data::fixture::path("fixtures/m169/eu-rng.bin")) else {
            eprintln!("skipped: no fixtures/m169/eu-rng.bin");
            return;
        };
        let Ok(m) = std::fs::read(hst_data::fixture::path("fixtures/m169/eu-bot-match.bin")) else { return };
        let frames = hst_sim::replay::frames_live(&m);
        let start = frames.iter().find(|f| f.gm()[0x55] == 2).unwrap().vsync() as i64;
        let rec: Vec<(i64, Mt, Mt, Mt)> = d
            .as_chunks::<{ 4 + 3 * 0x9d0 }>()
            .0
            .iter()
            .map(|s| (u32::from_le_bytes(s[..4].try_into().unwrap()) as i64 - start, Mt::from_ram(&s[4..]), Mt::from_ram(&s[4 + 0x9d0..]), Mt::from_ram(&s[4 + 2 * 0x9d0..])))
            .collect();
        // `HST_M183A1_KEEP=n`: play n ticks (the probes below reach past the asserted span)
        let keep = std::env::var("HST_M183A1_KEEP").ok().and_then(|v| v.parse().ok()).unwrap_or(470);
        let Some((_, _, rngs)) = european_rngs(Some(keep), 0, keep) else { return };
        let mut last = None;
        for (k, r) in rngs.iter().enumerate() {
            let at = |f: &dyn Fn(&(i64, Mt, Mt, Mt)) -> bool| rec.iter().find(|x| f(x)).map(|x| x.0);
            let got = (at(&|x| x.1 == r.shared), at(&|x| x.2 == r.ai), at(&|x| x.3 == r.court));
            if Some(got) != last {
                eprintln!("eu rng tick {k}: shared at {:?}, ai at {:?}, court at {:?}", got.0, got.1, got.2);
                last = Some(got);
            }
        }
        // draws a tick around the return: the recording's and play's (a tick later)
        let steps = |a: &Mt, b: &Mt| {
            let mut g = a.clone();
            (0..3000).find(|_| {
                let hit = g == *b;
                g.next();
                hit
            })
        };
        // `HST_M183A1_DRAWS=from,to`: another span of recorded ticks
        let span = std::env::var("HST_M183A1_DRAWS").ok().map(|v| v.split(',').filter_map(|x| x.parse::<usize>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 2).unwrap_or(vec![225, 266]);
        for k in span[0]..span[1].min(rngs.len() - 1) {
            let (Some(x0), Some(x1)) = (rec.iter().find(|x| x.0 == k as i64 - 1), rec.iter().find(|x| x.0 == k as i64)) else { continue };
            eprintln!("eu ai draws at recorded {k}: game {:?}, play {:?}", steps(&x0.2, &x1.2), steps(&rngs[k].ai, &rngs[k + 1].ai));
        }
        // the AI generator as recorded every tick through player 2's steps after the return and the ticks after it
        // stops (250-260; play a tick later), its stand to the server's return back (303), the receiver's whiff and
        // its release (407, M183a1), the rally's end (the point decided two ticks after the deciding bounce, B723) and
        // the point-over phase to the scoreboard's pause end (463, before the game's first loading stall), but at
        // the set-up's first sample and the serve's contact (140), where play's draws land a tick apart from the game's
        let off: Vec<usize> = (2..=463).filter(|&k| k != 140).filter(|&k| rec.iter().find(|x| x.0 == k as i64).is_some_and(|x| x.2 != rngs[k + 1].ai)).collect();
        assert!(off.is_empty(), "the AI generator leaves the recording at {off:?}");
    }

    /// Slot 5's bot match (`fixtures/match_s05.bin`: doubles, characters 0/2/1/5, court 10, umpire 4, 1 set of 4
    /// games) in play at 60 Hz from its match seed, with the generators after each of the first `keep` ticks.
    fn us(n: Option<usize>, keep: usize, toss_ai: usize) -> Option<(Vec<Tick>, Vec<Tick>, Vec<hst_sim::rng::Rngs>)> {
        let Ok(data) = hst_data::fixture::read("fixtures/match_s05.bin") else {
            eprintln!("skipped: no match_s05.bin");
            return None;
        };
        let Ok(rng) = hst_data::fixture::read("fixtures/p3b/rng_s05.bin") else {
            eprintln!("skipped: no rng_s05.bin");
            return None;
        };
        if !hst_data::fixture::has_iso() {
            return None;
        }
        let _turn = super::EU_DISC.lock().unwrap_or_else(|e| e.into_inner());
        let args = Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 10, stage: Some(10), play: true, singles: false, chars: vec![0, 2, 1, 5], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None };
        let size = 4 + 8 + 4 * 0x9d0;
        let rand_at = |v: u32| rng.chunks_exact(size).find(|s| u32::from_le_bytes(s[..4].try_into().unwrap()) == v).map(|s| u64::from_le_bytes(s[4..12].try_into().unwrap())).unwrap();
        // The match seed is the `rand()` output 0x28c7c4a1 (P3d3, `weather.rs`), but the intro was cut short when slot
        // 5 was made (a button skips it): `rand()` stands 2642 draws past the seed as the first point starts, where a
        // whole intro's lens flare (394 ticks of 24 draws) takes 9456. So `rand()` starts where play's setup (the
        // setup's 50 draws, the sound manager's reseed, the whole intro) lands on the recording's `rand()` the frame
        // before the first point's reseed (vsync 7565); the shared generator keeps the seed the schedule came from.
        let rand = hst_sim::rng::Rand(rand_back(rand_at(7565), 50 + 1 + 394 * 24));
        let seed = hst_sim::rng::Seed { rand, shared: 0x28c7_c4a1 };
        let want = recorded_ticks(&data);
        let (got, rngs) = played_ticks_rng(args, seed, n.unwrap_or(want.len() + 10), keep, toss_ai);
        Some((want, got, rngs))
    }

    /// The US baseline for the European test above: slot 5's first serve as recorded (M184's tick table): toss press
    /// at 124, the ball leaves the hand at 169 (45 ticks on), the press at 215, the contact at 219; players, the held
    /// ball and the toss bit for bit to the contact. And the generators, beside `rng_s05.bin`: the toss's hit message
    /// redraws each computer player's timing errors and picks (60 AI draws in all, with the launch's 2 shared ones),
    /// the press draws the swing kind, the contact the aim and the launch.
    #[test]
    fn the_us_first_serve_set_up_plays_as_recorded() {
        let Some((want, got, rngs)) = us(Some(380), 240, 0) else { return };
        serve_to_contact("us", &want, &got, 9, (169, 215, 219));
        serve_to_return("us", &want, &got, 219, 370, 298);
        // to the tick before the server's side strikes it back; every player stands where the recording has them to
        // there (they part at 855 first, `the_us_bot_match_plays_as_recorded`)
        return_flight("us", &want, &got, 298, 370);
        // the returner's slide into the return (the contact IK's body step, 8 ticks from 291): solved for the path
        // record's contact, a few millionths off the ball that arrives, bit for bit
        for k in 291..=298 {
            let (w, g) = (want[k].1[1], got[k + 1].1[1]);
            assert!(w[2] == g[2] && w[0] == g[0], "us tick {k}: the returner's slide {w:?} vs {g:?}");
        }
        let Ok(d) = hst_data::fixture::read("fixtures/p3b/rng_s05.bin") else { return };
        let size = 4 + 8 + 4 * 0x9d0;
        let rec = |k: u32| {
            let s = d.chunks_exact(size).find(|s| u32::from_le_bytes(s[..4].try_into().unwrap()) == 7566 + k).unwrap();
            (u64::from_le_bytes(s[4..12].try_into().unwrap()), hst_sim::rng::Mt::from_ram(&s[12..]), hst_sim::rng::Mt::from_ram(&s[12 + 0x9d0..]))
        };
        for k in [123, 124, 168, 169, 214, 215, 219, 220] {
            let (rand, shared, ai) = rec(k);
            let r = &rngs[k as usize + 1];
            assert!(r.rand.0 == rand && r.shared == shared && r.ai == ai, "us tick {k}: the generators");
        }
    }

    /// The US baseline for the European probe, compared the same way, so a 50 Hz gap can be told from one that plays
    /// the US match wrong too; with the generators beside slot 5's recorded ones.
    #[test]
    #[ignore]
    #[allow(clippy::needless_range_loop)]
    fn the_us_bot_match_plays_as_recorded() {
        let toss_ai = std::env::var("HST_M183A1_TOSS_AI").ok().and_then(|n| n.parse().ok()).unwrap_or(0);
        let Some((want, got, rngs)) = us(None, 1500, toss_ai) else { return };
        // B723: the first point's serve set-up, rally and point-over phase as long as the game's (point 2's set-up
        // parts: B722)
        // M183a1: point 2's set-up too (the flare draws before the point's logic, the reaction stops the players
        // and the AI routines from the scoreboard's pause end), and every player on the ground from the first serve
        // to point 2's serve
        assert!(compare("us", &want, &got) >= 4, "the first point's and point 2's set-up phases");
        let ground = |t: &Tick| t.1.map(|p| [p[0], p[2]]);
        // (but at the point-over phase's last tick, where play, a tick behind, has already placed them for the set-up)
        // B727: and on through point 2's serve and rally (the toss heard as a hit by nobody, the serve history kept)
        assert_eq!((219..2354).filter(|&k| k != 1084).find(|&k| ground(&want[k]) != ground(&got[k + 1])), None, "the players part before tick 2354");
        first_point_part("us", &want, &got, 219);
        part_from("us", &want, &got);
        // B720: after the server's partner's dive every player stands where the recording has them and the ball flies
        // as recorded through the point (B722: player 1's smash at 856 steps it sideways, the smash's contact IK)
        let ground = |t: &Tick| t.1.map(|p| [p[0], p[2]]);
        let end = want.iter().skip(219).position(|t| t.0 != 3).map_or(want.len(), |n| n + 219);
        assert_eq!((219..end).find(|&k| ground(&want[k]) != ground(&got[k + 1])), None, "the players part");
        assert_eq!((220..end).find(|&k| want[k].2 != got[k].2), None, "the ball parts");
        if let Some(v) = std::env::var_os("HST_M183A1_DUMP") {
            // `from,to`: another span of recorded ticks
            let span: Vec<usize> = v.to_string_lossy().split(',').filter_map(|x| x.parse().ok()).collect();
            let (from, to) = if span.len() == 2 { (span[0], span[1]) } else { (0, 240) };
            for k in from..to.min(want.len()) {
                eprintln!("us dump {k}: want {} {:?} {:?} | got {} {:?} {:?}", want[k].0, want[k].1[0], want[k].2, got[k + 1].0, got[k + 1].1[0], got[k + 1].2);
            }
        }
        // the generators beside slot 5's recorded ones (`fixtures/p3b/rng_s05.bin`, vsyncs 7533..9032; the serve
        // set-up starts at 7566): each played tick's rand / shared / AI generator, at the recorded vsync it equals
        if let Ok(d) = hst_data::fixture::read("fixtures/p3b/rng_s05.bin") {
            use hst_sim::rng::Mt;
            let size = 4 + 8 + 4 * 0x9d0;
            let rec: Vec<(u32, u64, Mt, Mt)> = d
                .chunks_exact(size)
                .map(|s| (u32::from_le_bytes(s[..4].try_into().unwrap()), u64::from_le_bytes(s[4..12].try_into().unwrap()), Mt::from_ram(&s[12..]), Mt::from_ram(&s[12 + 0x9d0..])))
                .collect();
            // how many `rand()` draws play's is from the recording's, at play's setup and first ticks
            for k in [0usize, 1, 2, 3, 10] {
                let (mine, theirs) = (rngs[k].rand, rec.iter().find(|x| x.0 == 7565 + k as u32).unwrap().1);
                let ahead = |from: u64, to: u64| {
                    let mut r = hst_sim::rng::Rand(from);
                    (0..2_000_000).find(|_| {
                        r.next();
                        r.0 == to
                    })
                };
                eprintln!("us rand tick {k}: play's is {:?} draws after the recording's, or {:?} before", ahead(theirs, mine.0).map(|n| n + 1), ahead(mine.0, theirs).map(|n| n + 1));
            }
            // the AI generator around the serve's press and contact: draws from its state at vsync 7566 + 124
            let ai124 = rec.iter().find(|x| x.0 == 7566 + 124).unwrap().3.clone();
            let draws = |to: &Mt| {
                let mut g = ai124.clone();
                (0..400).find(|_| {
                    let hit = g == *to;
                    g.next();
                    hit
                })
            };
            let mut last = None;
            for x in rec.iter().filter(|x| (7566 + 124..7566 + 300).contains(&x.0)) {
                let n = draws(&x.3);
                if n != last {
                    eprintln!("us ai recorded at {}: {n:?} draws past 124", x.0 as i64 - 7566);
                    last = n;
                }
            }
            for (k, r) in rngs.iter().enumerate().take(230).skip(214) {
                eprintln!("us ai play tick {k}: {:?} draws past 124", draws(&r.ai));
            }
            // M185: the AI generator as recorded every tick through point 1's return and the server's partner's stand
            // searches (which read the previous player's racket, `doubles_ai::scratch`) and the stands and walks
            // back after the return (B719), the server's partner's walk-back wait and re-roll after its dive (its
            // stroke flag held 5 ticks past the dive, B720) and the rest of the rally (player 1's smash step, B722) to
            // the two ticks before the recording ends it (play ends the rally two ticks early: B723), but at the serve's
            // contact (218, two draws early, as before)
            let off: Vec<usize> = (2..end - 2).filter(|&k| rec.iter().find(|x| x.0 == 7566 + k as u32).is_some_and(|x| x.3 != rngs[k + 1].ai)).collect();
            assert_eq!(off, [218], "the AI generator leaves the recording at {off:?}");
            let lost = (2..end - 2).filter(|&k| k != 218).find(|&k| !rec.iter().any(|x| x.3 == rngs[k + 1].ai));
            assert_eq!(lost, None, "play's AI generator is off the recording's draws");
            // B727: point 2's toss heard as a hit by nobody (the next-point step clears the last hitter) and the doubles
            // set-up keeping the serve history after the match's first: play's AI generator stays on the recording's
            // draws through point 2's set-up, serve and rally to where the recording ends (its last few ticks are
            // ones play, a few ticks ahead here, has passed), but at the two serves' contacts (their aim draws a tick
            // early) and point 1's point-over tick 1086
            let last = rec.last().unwrap().0 as usize - 7566;
            let lost: Vec<usize> = (2..last - 6).filter(|&k| !rec.iter().any(|x| x.3 == rngs[k + 1].ai)).collect();
            assert_eq!(lost, [218, 1086, 1269], "play's AI generator is off the recording's draws through point 2");
            // `HST_M183A1_SHARED=from,to`: for each played tick, how many shared draws play's generator is from the
            // recorded one at the vsync rand puts it at (+: play ahead)
            if let Some(v) = std::env::var("HST_M183A1_SHARED").ok().map(|v| v.split(',').filter_map(|x| x.parse::<usize>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 2) {
                for k in v[0]..v[1].min(rngs.len()) {
                    let Some(x) = rec.iter().find(|x| x.1 == rngs[k].rand.0) else { continue };
                    let ahead = |from: &Mt, to: &Mt| {
                        let mut g = from.clone();
                        (0..3000).find(|_| {
                            let hit = g == *to;
                            g.next();
                            hit
                        })
                    };
                    let (a, b) = (ahead(&x.2, &rngs[k].shared), ahead(&rngs[k].shared, &x.2));
                    let (c, d) = (ahead(&x.3, &rngs[k].ai), ahead(&rngs[k].ai, &x.3));
                    eprintln!("us shared tick {k} (vsync {}): play ahead {a:?} behind {b:?}; ai ahead {c:?} behind {d:?}", x.0 as i64 - 7566);
                }
            }
            // `HST_M183A1_SEED=k`: which `rand()` output (draws before play's tick k's rand) seeds play's and the
            // recording's shared generator at tick k, and how many draws it has made since
            if let Some(k) = std::env::var("HST_M183A1_SEED").ok().and_then(|v| v.parse::<usize>().ok()) {
                let x = rec.iter().find(|x| x.1 == rngs[k].rand.0).unwrap();
                for (who, target) in [("play", &rngs[k].shared), ("recorded", &x.2)] {
                    let mut r = hst_sim::rng::Rand(rand_back(rngs[k].rand.0, 3000));
                    let hit = (0..3000).find_map(|n| {
                        let o = r.next();
                        let mut g = Mt::new(o);
                        (0..60).find(|_| {
                            let hit = g == *target;
                            g.next();
                            hit
                        }).map(|d| (3000 - n, d))
                    });
                    eprintln!("us seed tick {k}: {who}'s shared is the output {:?} (draws before, then draws made)", hit);
                }
            }
            // `HST_M183A1_AIDRAWS=from,to`: the AI generator's draws since recorded vsync `from` − 10, for play's ticks
            // (beside the vsync rand puts them at) and the recorded vsyncs
            if let Some(v) = std::env::var("HST_M183A1_AIDRAWS").ok().map(|v| v.split(',').filter_map(|x| x.parse::<usize>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 2) {
                let base = rec.iter().find(|x| x.0 == 7566 + v[0] as u32 - 10).unwrap().3.clone();
                let count = |to: &Mt| {
                    let mut g = base.clone();
                    (0..2000).find(|_| {
                        let hit = g == *to;
                        g.next();
                        hit
                    })
                };
                for k in v[0]..v[1].min(rngs.len()) {
                    let x = rec.iter().find(|x| x.1 == rngs[k].rand.0);
                    let r = rec.iter().find(|x| x.0 == 7566 + k as u32);
                    eprintln!("us aidraws play tick {k} (at vsync {:?}): {:?} | recorded vsync {k}: {:?}", x.map(|x| x.0 as i64 - 7566), count(&rngs[k].ai), r.and_then(|r| count(&r.3)));
                }
            }
            for (k, r) in rngs.iter().enumerate() {
                let at = |f: &dyn Fn(&(u32, u64, Mt, Mt)) -> bool| rec.iter().find(|x| f(x)).map(|x| x.0 as i64 - 7566);
                let (rand, shared, ai) = (at(&|x| x.1 == r.rand.0), at(&|x| x.2 == r.shared), at(&|x| x.3 == r.ai));
                // `HST_M183A1_RNG=from,to`: every played tick of another span
                let span = std::env::var("HST_M183A1_RNG").ok().map(|v| v.split(',').filter_map(|x| x.parse::<usize>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 2);
                if k < 12 || k % 20 == 0 || (205..226).contains(&k) || span.is_some_and(|v| (v[0]..v[1]).contains(&k)) {
                    eprintln!("us rng tick {k}: rand at {rand:?}, shared at {shared:?}, ai at {ai:?}");
                }
            }
        }
    }

    /// One-off (M183a1): which `rand()` output seeded the European match's weather schedule, the one in
    /// `eu-bot-late_ee.bin` (match object +0xd8 weathers, +0x11c speed and direction pairs), walking back from the
    /// 3–3 point's reseed (the 875 477th output).
    #[test]
    #[ignore]
    fn eu_match_seed_search() {
        let iso = hst_data::fixture::path(EU);
        let Ok(ram) = std::fs::read(hst_data::fixture::path("fixtures/m169/eu-bot-late_ee.bin")) else { return };
        let u = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap());
        let gm = u(0x437e00) as usize;
        let mut disc = hst_data::iso::Iso::open(&iso).unwrap();
        let (cnf, bin) = (disc.read("SYSTEM.CNF").unwrap(), disc.read("ZZBIN/GAME.BIN").unwrap());
        let exe = hst_data::exe::Game::new(&cnf, &bin).unwrap();
        let o = exe.weather_odds(6);
        let odds = hst_sim::weather::Odds { cloudy: o[0], cloudy_len: [o[1], o[2]], rain: o[3], rain_len: [o[4], o[5]], no_border: o[7] != 0, heavy: o[8] != 0 };
        let ((directions, speed), (chance, kind)) = (exe.wind(6), exe.gusts(6));
        let wind = hst_sim::weather::Wind { chance, kind, directions, speed };
        let want: Vec<(u8, u32, u32)> = (0..65).map(|i| (ram[gm + 0xd8 + i], u(gm + 0x11c + 8 * i), u(gm + 0x120 + 8 * i))).collect();
        let mut st = hst_sim::rng::Seed::after(875_476).rand.0;
        for back in 0..875_477u32 {
            let out = (st >> 32) as u32 & 0x7fff_ffff;
            let mut mt = hst_sim::weather::Mt::new(out);
            let s = hst_sim::weather::schedule(&odds, &wind, 4, 1, 4, || mt.next());
            if s.iter().zip(&want).all(|(g, w)| (g.weather, g.speed.to_bits(), g.degrees as i32 as u32) == *w) {
                eprintln!("eu seed: output {out:#x}, {back} outputs back, the {}th from boot", 875_477 - back);
                return;
            }
            st = rand_back(st, 1);
        }
        panic!("no seed");
    }

    /// newlib's `rand()` state `n` draws before `state`.
    fn rand_back(state: u64, n: u32) -> u64 {
        const A: u64 = 0x5851_f42d_4c95_7f2d;
        // A's inverse mod 2^64 (Newton: each step doubles the bits that are right)
        let inv = (0..6).fold(A, |x, _| x.wrapping_mul(2u64.wrapping_sub(A.wrapping_mul(x))));
        (0..n).fold(state, |x, _| x.wrapping_sub(1).wrapping_mul(inv))
    }

    impl Game {
        fn clone_phase(&self) -> (Phase, u32, bool) {
            (self.phase.clone(), self.shots as u32, self.serving.tossed)
        }
    }
}

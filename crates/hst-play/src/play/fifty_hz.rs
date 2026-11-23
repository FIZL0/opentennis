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

/// B785 probe stand-in: the next umpire line play starts is heard to stop a tick early (its ticks left one shorter).
/// The sound processor isn't in step with the frames, so now and then the game hears a line's end a tick sooner than
/// its length says; nothing in the game's state tells when, so the whole-match proof feeds the recording's early
/// lines (`EARLY_LINES`). Process-wide, as Bevy runs the match on its own threads; set only under `EU_DISC`.
#[cfg(test)]
pub(super) static EARLY_LINE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, crate::play::swing_after_step::apply, played_out, held_ball, crate::play::bodyhit::pose, crate::play::doubles_ai::scratch::after_pose).chain());
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
                // B766: a phase's first sample can come before its first tick has run (counter 0, at 30-30,
                // 30-40, ...): it counts as no tick, and the next sample's count repeats it for tick 1
                out.extend(std::iter::repeat_n(b, tick(&w[1]) as usize));
            }
        }
        out
    }

    /// B768: for each tick of [`recorded_ticks`], the vsync (from the first serve set-up's) its sample was taken at,
    /// `None` for a tick that ran unsampled (a repeat). Recordings indexed by vsync (the generators') read through it:
    /// the game's loading stalls are vsyncs with no tick, so past the first one vsync and tick numbers drift apart.
    fn recorded_vsyncs(data: &[u8]) -> Vec<Option<i64>> {
        let frames = hst_sim::replay::frames_live(data);
        let first = frames.iter().position(|f| f.gm()[0x55] == 2).unwrap();
        let start = frames[first].vsync() as i64;
        let tick = |f: &hst_sim::replay::Frame| u32::from_le_bytes(f.gm()[0x58..0x5c].try_into().unwrap()) as usize;
        let mut out = vec![Some(0)];
        for w in frames[first..].windows(2) {
            // as `recorded_ticks` counts them: in a phase the counter's step, at a phase change the new count
            let n = if w[0].gm()[0x55] == w[1].gm()[0x55] { tick(&w[1]).saturating_sub(tick(&w[0])) } else { tick(&w[1]) };
            if n > 0 {
                out.extend(std::iter::repeat_n(None, n - 1));
                out.push(Some(w[1].vsync() as i64 - start));
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
    type ShotProbe = (hst_sim::ball::Ball, u8, i32, f32, f32, [f32; 3], [f32; 3], [[f32; 2]; 4]);
    thread_local! {
        /// M183a1 probe: after each played tick, the live ball (position, velocity, spin) and the shot's class,
        /// kind, curve, bend and side (`HST_M183A1_SHOT`).
        static SHOTS: std::cell::RefCell<Vec<ShotProbe>> = const { std::cell::RefCell::new(Vec::new()) };
        /// B775: each player's motion and clock (its next sampled time, and the time it sampled last) in the game's
        /// start state, for [`played_ticks_rng`] to start from instead of play's set-up (none: as play sets them up).
        static START_CLOCKS: std::cell::Cell<Option<[(usize, f32, f32); 4]>> = const { std::cell::Cell::new(None) };
        /// B785: the recorded ticks of the point-overs whose call the recording hears stop a tick early (the line's
        /// end follows the sound processor, not the game's state), for [`played_ticks_rng`] to feed play
        /// (`EARLY_LINE`); none: every line ends on its usual tick.
        static EARLY_LINES: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
        /// M183a1 probe: the live ball's whole flight after each played tick in `HST_M183A1_FLIGHT=from,to`.
        static FLIGHTS: std::cell::RefCell<Vec<(usize, hst_sim::ball::Flight)>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    /// `HST_M183A1_FLIGHT=from,to`: the span of recorded ticks [`flight_probe`] lists.
    fn flight_span() -> Option<(usize, usize)> {
        std::env::var("HST_M183A1_FLIGHT").ok().map(|v| v.split(',').filter_map(|x| x.parse::<usize>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 2).map(|v| (v[0], v[1]))
    }

    /// Probe (`HST_M183A1_FLIGHT=from,to`): the recorded live ball's counts, slide, spin frame and contact frame
    /// beside play's (level), for each recorded tick.
    fn flight_probe(name: &str, file: &str) {
        let Some((from, to)) = flight_span() else { return };
        let Ok(data) = hst_data::fixture::read(file) else { return };
        let frames = hst_sim::replay::frames_live(&data);
        let want = recorded_ticks(&data);
        let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let rows = |b: &[u8], o: usize| -> [[f32; 4]; 4] { std::array::from_fn(|r| std::array::from_fn(|c| f(b, o + 16 * r + 4 * c))) };
        let flights = FLIGHTS.with_borrow(|v| v.clone());
        for (k, w) in want.iter().enumerate().take(to).skip(from) {
            let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
            if let Some(fr) = frames.iter().find(|fr| v3(fr.live_ball(), 0xe0) == w.2) {
                let b = fr.live_ball();
                eprintln!("{name} flight {k} want: counts {:?} slide {} rolling {} fbs {} spin frame {:?} contact {:?}", (i(b, 0xac), i(b, 0x224), i(b, 0x228), i(b, 0x22c)), f(b, 0x200), b[0xa4] == 2, f(b, 0x1a8), rows(b, 0x160), rows(b, 0x1c0));
            }
            if let Some((_, fl)) = flights.iter().find(|(t, _)| *t == k) {
                eprintln!("{name} flight {k} got:  counts {:?} slide {} rolling {} fbs - spin frame {:?} contact {:?}", (fl.frame, fl.bounces, fl.contacts, fl.special_contacts), fl.slide, fl.rolling, fl.spin_frame, fl.contact);
            }
        }
    }

    /// B775: the recording's start state on the players play just set up: a bot match starts from the game's frame
    /// before the serve set-up, where each player already plays a looping motion its last command started (the server
    /// its set-up motion, the others their idle) and has run it on from the screens before. So each player gets that
    /// motion at the recorded clock, as its standing command, and the set-up's commands for the same motion keep it
    /// going (the motion setter restarts only a player's first command).
    fn start_clocks(app: &mut App) {
        let Some(start) = START_CLOCKS.take() else { return };
        let mut q = app.world_mut().query::<(&crate::play::Figure, &mut Motion)>();
        let mut set = Vec::new();
        for (f, mut m) in q.iter_mut(app.world_mut()) {
            let (id, time, sampled) = start[f.0];
            let clock = hst_sim::motion::Clock { time, sampled, ..hst_sim::motion::Clock::start(1.0, true, None) };
            (m.id, m.face, m.clock, m.face_clock, m.uv_clock, m.drawn, m.serial) = (id, id, clock, clock, clock, clock, 1);
            set.push((f.0, id));
        }
        let mut g = app.world_mut().resource_mut::<Game>();
        for (i, id) in set {
            g.players[i].cmd = crate::play::Cmd { id, speed: 1.0, looping: true, hold: None, serial: 1 };
        }
    }

    /// The match's setup as main.rs makes it from `seed`: the weather schedule, the generators after the setup's
    /// draws (B782: its clouds, remade at the first weather as the recordings' setups did, `SETUP_MADE`) and the
    /// effects seed.
    fn match_setup(args: &Args, seed: hst_sim::rng::Seed) -> ([hst_sim::weather::Game; hst_sim::weather::GAMES], hst_sim::rng::Rngs, u32) {
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
        let sky = crate::match_sky(&mut disc, args.court as u32, args.singles, schedule.map(|g| g.weather), None, exe.rain_clouds().0);
        let (effects, _) = rngs.setup_rand(crate::setup_clouds(sky.as_ref(), args.singles, crate::SETUP_MADE));
        (schedule, rngs, effects)
    }

    fn played_ticks_rng(args: Args, seed: hst_sim::rng::Seed, n: usize, keep: usize, toss_ai: usize) -> (Vec<Tick>, Vec<hst_sim::rng::Rngs>) {
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        let (schedule, rngs, effects) = match_setup(&args, seed);
        set_effects_seed(effects);
        app.insert_resource(MatchRng(rngs));
        app.insert_resource(crate::weather::Weather { schedule, game: 0, fixed: None });
        app.insert_resource(args);
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, flare_tick, effects::tick, react_first, control, reaction_roots, simulate, start_effects, motions, character::tick, crate::play::swing_after_step::apply, played_out, held_ball, crate::play::bodyhit::pose, crate::play::doubles_ai::scratch::after_pose).chain());
        app.init_resource::<ReactionRoots>();
        // B769: the umpire's calls are voiced, so the point-over waits for the voice as in a real match (a fault's
        // point-over is 80 ticks in the game, 38 without a mixer)
        let mut iso = hst_data::fixture::iso().unwrap();
        app.insert_resource(crate::audio::Sound::test(&mut iso));
        app.add_plugins(heard::plugin);
        app.add_systems(FixedUpdate, play_sounds.after(held_ball));
        // B770: the court's figures (court 6's fences stop the ball and send the computer players the strike)
        app.add_plugins(crate::play::npcs::sim_plugin);
        app.init_resource::<crate::play::bodyhit::Posed>().init_resource::<crate::play::bodyhit::Bones>();
        app.update();
        // M183a1: the ball hitting a player ends the point (phase 38's rally ends on the net player's body)
        crate::play::bodyhit::sim_plugin(&mut app);
        start_clocks(&mut app);
        let code = |p: &Phase| match p {
            Phase::Serve => 2,
            Phase::Rally => 3,
            Phase::Post => 4,
            Phase::ChangeEnds(_) => 1,
        };
        let tick = |g: &Game| (code(&g.phase), std::array::from_fn(|i| g.players[i].pos), g.flight.ball.pos);
        let mut out = vec![tick(app.world().resource::<Game>())];
        let mut rngs = vec![app.world().resource::<Game>().rng.clone()];
        let early = EARLY_LINES.take();
        for k in 1..n {
            let tossed = app.world().resource::<Game>().serving.tossed;
            // B785: a listed point-over's call heard a tick early (play starts the call in its step t, the
            // recording's point-over tick t); the line play starts next takes it
            if early.contains(&k) {
                super::EARLY_LINE.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            app.world_mut().run_schedule(FixedUpdate);
            let mut g = app.world_mut().resource_mut::<Game>();
            if g.serving.tossed && !tossed {
                (0..toss_ai).for_each(|_| _ = g.rng.ai.next());
            }
            out.push(tick(app.world().resource::<Game>()));
            SHOTS.with_borrow_mut(|v| {
                let g = app.world().resource::<Game>();
                v.push((g.flight.ball, g.shot.class, g.shot.kind, g.shot.curve, g.shot.bend, g.shot.side, g.aim.target, std::array::from_fn(|i| g.players.get(i).map_or([0.0; 2], |p| p.aim_from))));
            });
            if flight_span().is_some_and(|(a, b)| (a..b).contains(&k)) {
                FLIGHTS.with_borrow_mut(|v| v.push((k, app.world().resource::<Game>().flight)));
            }
            if k < keep {
                rngs.push(app.world().resource::<Game>().rng.clone());
            }
        }
        super::EARLY_LINE.store(false, std::sync::atomic::Ordering::Relaxed);
        (out, rngs)
    }

    /// The two side by side: the first phases, how many match in kind and length, and the first tick a player stands
    /// elsewhere. Returns the phases that match.
    fn compare(name: &str, want: &[Tick], got: &[Tick]) -> usize {
        let (w, g) = (phases(want), phases(got));
        // `HST_M183A1_PHASES=n`: list n phases (default 30)
        let listed = std::env::var("HST_M183A1_PHASES").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
        for (k, (w, g)) in w.iter().zip(&g).enumerate().take(listed) {
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

    /// Probe (`HST_M183A1_SHOT=from,to`): for each recorded tick, the recorded live ball's velocity, spin, shot class,
    /// kind, curve, bend and side (the frame whose ball sits where the tick's does) beside play's (level).
    #[allow(clippy::needless_range_loop)]
    fn shot_probe(name: &str, file: &str, want: &[Tick]) {
        let Some(v) = std::env::var("HST_M183A1_SHOT").ok().map(|v| v.split(',').filter_map(|x| x.parse::<usize>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 2) else { return };
        let Ok(data) = hst_data::fixture::read(file) else { return };
        let frames = hst_sim::replay::frames_live(&data);
        let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
        let shots = SHOTS.with_borrow(|s| s.clone());
        for k in v[0]..v[1].min(want.len()) {
            if let Some(fr) = frames.iter().find(|fr| v3(fr.live_ball(), 0xe0) == want[k].2) {
                let b = fr.live_ball();
                eprintln!("{name} shot {k} want: vel {:?} spin {} class {} kind {} curve {} bend {} side {:?}", v3(b, 0x130), f(b, 0x1a4), b[0x58], i32::from_le_bytes(b[0x5c..0x60].try_into().unwrap()), f(b, 0x254), f(b, 0x250), v3(b, 0x90));
                // each player's aim result and swing-start point (+0x3e90, +0x3ef0: M187)
                let aims: Vec<_> = (0..4).map(|i| ([fr.player_f32(i, 0x3e90), fr.player_f32(i, 0x3e98)], [fr.player_f32(i, 0x3ef0), fr.player_f32(i, 0x3ef8)])).collect();
                eprintln!("{name} shot {k} want aims {aims:?}");
                let pi = |i: usize, o: usize| fr.player_f32(i, o).to_bits() as i32;
                let hit: Vec<_> = (0..4).map(|i| (fr.player_f32(i, 0x3ec0).to_bits().to_le_bytes()[1], pi(i, 0x3ee4), pi(i, 0x3fa0), pi(i, 0x3f50) & 4 != 0, fr.player_f32(i, 0x3f44))).collect();
                eprintln!("{name} shot {k} want hitters (branch, kind flags, offset, body, height) {hit:?}");
            }
            // shots[j] is played tick j + 1
            if let Some((ball, class, kind, curve, bend, side, aim, from)) = k.checked_sub(1).and_then(|j| shots.get(j)) {
                eprintln!("{name} shot {k} got:  vel {:?} spin {} class {class} kind {kind} curve {curve} bend {bend} side {side:?}", ball.vel, ball.spin);
                eprintln!("{name} shot {k} got aim {:?} from {from:?}", [aim[0], aim[2]]);
            }
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
        // B778: the motions as the recording's start state has them (`eu-bot-anim.bin`, vsync 37921, the frame before
        // the first set-up): the server in its serve stance and the others in their idle, all run on from the screens
        // before to the same fractional 50 Hz clock (23.6664, sampled 22.6664)
        let (time, sampled) = (f32::from_bits(0x41bd_54ce), f32::from_bits(0x41b5_54ce));
        START_CLOCKS.set(Some([(0x20, time, sampled), (0, time, sampled), (0, time, sampled), (0, time, sampled)]));
        // B785: the match's first fault (point-over from recorded tick 6636) hears its call stop a tick early
        EARLY_LINES.set(vec![6636]);
        // B782: the recording's setup remade court 6's 6 doubles clouds at its first weather (30 `rand()` draws) and
        // its intro ran the 50 Hz camera table's 327 flare ticks, as play makes them
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
        serve_to_contact("eu", &want, &got, 0, (114, 134, 141));
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
    /// as recorded to the next serve's set-up (571), off court 6's fence at 471 too (B770).
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
        for k in (437..=571).filter(|&k| k != 464) {
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
        // to the end of the match (the result after it is a screen of ours)
        assert!(compare("eu", &want, &got) >= 144, "the phases through the last point");
        first_point_part("eu", &want, &got, 141);
        part_from("eu", &want, &got);
        shot_probe("eu", "fixtures/m169/eu-bot-match.bin", &want);
        flight_probe("eu", "fixtures/m169/eu-bot-match.bin");
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
    /// follows the recording, read by game tick (`recorded_vsyncs`), through the first point, the ball striking court
    /// 6's fence (471, B770), point 2 and on to point 3's serve, and the phases with it.
    #[test]
    fn the_european_generators_play_as_recorded() {
        use hst_sim::rng::Mt;
        // the longer recording (7000 ticks from the same start, the first 1300 the same as eu-rng.bin) when it's there
        let long = std::fs::read(hst_data::fixture::path("fixtures/m183a1/eu-rng-full.bin")).or_else(|_| std::fs::read(hst_data::fixture::path("fixtures/m183a1/eu-rng-long.bin")));
        let Ok(d) = long.or_else(|_| std::fs::read(hst_data::fixture::path("fixtures/m169/eu-rng.bin"))) else {
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
        let keep = std::env::var("HST_M183A1_KEEP").ok().and_then(|v| v.parse().ok()).unwrap_or(1190);
        let Some((want, got, rngs)) = european_rngs(Some(keep), 0, keep) else { return };
        let mut last = None;
        for (k, r) in rngs.iter().enumerate() {
            let at = |f: &dyn Fn(&(i64, Mt, Mt, Mt)) -> bool| rec.iter().find(|x| f(x)).map(|x| x.0);
            let got = (at(&|x| x.1 == r.shared), at(&|x| x.2 == r.ai), at(&|x| x.3 == r.court));
            if Some(got) != last {
                eprintln!("eu rng tick {k}: shared at {:?}, ai at {:?}, court at {:?}", got.0, got.1, got.2);
                last = Some(got);
            }
        }
        // B768: the recorded generators by game tick, at the vsync each tick's sample was taken (the loading stalls
        // are vsyncs with no tick: 2 behind from point 1's scoreboard pause end, 4 from point 2's set-up)
        let vsyncs = recorded_vsyncs(&m);
        let by_tick = |k: usize| vsyncs.get(k).copied().flatten().and_then(|v| rec.iter().find(|x| x.0 == v));
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
        // `HST_B768_DRAWS=from,to`: the AI draws a tick by game tick, the recording's (where sampled) and play's
        let span = std::env::var("HST_B768_DRAWS").ok().map(|v| v.split(',').filter_map(|x| x.parse::<usize>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 2);
        for k in span.map_or(0..0, |v| v[0].max(1)..v[1].min(rngs.len() - 1)) {
            let game = by_tick(k - 1).zip(by_tick(k)).and_then(|(a, b)| steps(&a.2, &b.2));
            // how far play's AI stream is ahead of the game's (or behind it), and the same for the shared one
            let gap = |a: &Mt, b: &Mt| steps(a, b).map(|n| n as i64).or_else(|| steps(b, a).map(|n| -(n as i64)));
            let ahead = by_tick(k).map(|x| (gap(&x.2, &rngs[k + 1].ai), gap(&x.1, &rngs[k + 1].shared)));
            eprintln!("eu ai draws at tick {k} (vsync {:?}): game {game:?}, play {:?}, ahead {ahead:?}", vsyncs[k], steps(&rngs[k].ai, &rngs[k + 1].ai));
        }
        // the AI generator as recorded every tick through player 2's steps after the return and the ticks after it
        // stops (250-260; play a tick later), its stand to the server's return back (303), the receiver's whiff and
        // its release (407, M183a1), the rally's end (the point decided two ticks after the deciding bounce, B723) and
        // the point-over phase past the scoreboard's pause end and its loading stall to the ball meeting the stage
        // (470), but at the serve's contact (140), where play's draws land a tick apart from the game's; B768: the set-up's
        // first ticks too, as the server picks its serve spot and wait in its serve state's entry tick; point 2's
        // serve contact (742) and point 3's (1181) are a tick apart the same way
        let off: Vec<usize> = (1..=470).filter(|&k| k != 140).filter(|&k| by_tick(k).is_some_and(|x| x.2 != rngs[k + 1].ai)).collect();
        assert!(off.is_empty(), "the AI generator leaves the recording at {off:?}");
        // every tick has a recorded sample to compare with but the one run unsampled inside each loading stall
        let unsampled: Vec<usize> = (1..=652).filter(|&k| by_tick(k).is_none()).collect();
        assert_eq!(unsampled, [464, 572], "ticks with no recorded sample");
        // B768: point 2's new-point reset in the tick the set-up starts (572: each AI object's 17) and the entries and
        // the serve pick in the next (21), both inside the recording's one sample after the loading stall (vsync 577),
        // none after; B770: and on as recorded through the tick the game's ball meets court 6's fence (471: the
        // computer players' 6 draws as they hear the strike) to the server's toss (653: its serve wait)
        let drawn = |k: usize| steps(&rngs[k].ai, &rngs[k + 1].ai);
        assert_eq!((572..=574).map(drawn).collect::<Vec<_>>(), [Some(68), Some(21), Some(0)], "point 2's reset");
        assert_eq!(drawn(471), Some(6), "the strike on the fence");
        let off: Vec<usize> = (471..=1180).filter(|&k| k != 742).filter(|&k| by_tick(k).is_some_and(|x| x.2 != rngs[k + 1].ai)).collect();
        assert!(off.is_empty(), "past the fence, the AI generator leaves the recording at {off:?}");
        // so the match's phases run as recorded through point 3's set-up: point 2's set-up its 164 ticks, its rally
        // and point-over, point 3's set-up its 221 (they part in point 3's rally)
        let kinds = |p: &[(u8, u32, [[f32; 3]; 4])]| p.iter().take(7).map(|p| (p.0, p.1)).collect::<Vec<_>>();
        assert_eq!(kinds(&phases(&want)), [(2, 135), (3, 303), (4, 134), (2, 164), (3, 84), (4, 134), (2, 221)], "the recording's phases");
        assert_eq!(kinds(&phases(&got[..=1180])), kinds(&phases(&want)), "the phases in play");
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
        // setup's draws, the sound manager's reseed, the whole intro) lands on the recording's `rand()` the frame
        // before the first point's reseed (vsync 7565); the shared generator keeps the seed the schedule came from.
        // B782: the setup's draws counted as play makes them (its clouds remade at the first weather included)
        let setup = match_setup(&args, hst_sim::rng::Seed { rand: hst_sim::rng::Rand(0), shared: 0x28c7_c4a1 }).1.rand.0;
        let (mut r, mut draws) = (hst_sim::rng::Rand(0), 0);
        while r.0 != setup {
            r.next();
            draws += 1;
        }
        let rand = hst_sim::rng::Rand(rand_back(rand_at(7565), draws + 1 + 394 * 24));
        let seed = hst_sim::rng::Seed { rand, shared: 0x28c7_c4a1 };
        let want = recorded_ticks(&data);
        // B775: the motions as the recording's start state has them (`anim_s05.bin`, vsync 7565): the server in its
        // serve stance and the others in their idle, run on from the screens before (the last player's idle is a
        // longer clip, near its end), so the doubles stand test reads the poses the game reads on idle ticks
        START_CLOCKS.set(Some([(0x20, 2.0, 1.0), (0, 2.0, 1.0), (0, 2.0, 1.0), (0, 52.0, 51.0)]));
        let (got, rngs) = played_ticks_rng(args, seed, n.unwrap_or(want.len() + 10), keep, toss_ai);
        Some((want, got, rngs))
    }

    /// B783: slot 5's first set-up tick as it ends, in place of the recording's sample of it. The recorder took that
    /// sample (counter 1, the tick run after the loading stall) before the tick's motion step and ball placement had
    /// run: every player's clock and the held ball still read the frame before, and the next sample's are two steps on
    /// (`anim_s05.bin`, and a fresh record from slot 5 the same). The stance frame tick 0 ends on (the serve stance's
    /// third) comes round again at recorded tick 11: the server restarts its stance 0.2333 to the left at tick 9, and
    /// ticks 9 to 11 hold the ball where the stance's first three frames did, bit for bit apart from x. So tick 0's
    /// ball is tick 11's taken back by the server's step (as the hand's offset from the server: bit for bit the same).
    fn us_first_tick(want: &mut [Tick]) {
        let data = hst_data::fixture::read("fixtures/match_s05.bin").unwrap();
        let frames = hst_sim::replay::frames_live(&data);
        let first = frames.iter().position(|f| f.gm()[0x55] == 2).unwrap();
        let b = frames[first - 1].live_ball();
        let before = [0, 4, 8].map(|o| f32::from_le_bytes(b[0xe0 + o..0xe4 + o].try_into().unwrap()));
        // the stance moves the hand every frame: an unmoved ball is a tick sampled before its placement
        assert_eq!(want[0].2.map(f32::to_bits), before.map(f32::to_bits), "us: tick 0 sampled before its placement");
        assert_eq!(want[10].2[1..], before[1..], "us: tick 10 replays the frame before the set-up");
        let (from, to) = (want[11].1[0][0], want[0].1[0][0]);
        want[0].2 = [want[11].2[0] - from + to, want[11].2[1], want[11].2[2]];
    }

    /// The US baseline for the European test above: slot 5's first serve as recorded (M184's tick table): toss press
    /// at 124, the ball leaves the hand at 169 (45 ticks on), the press at 215, the contact at 219; players, the held
    /// ball and the toss bit for bit to the contact. And the generators, beside `rng_s05.bin`: the toss's hit message
    /// redraws each computer player's timing errors and picks (60 AI draws in all, with the launch's 2 shared ones),
    /// the press draws the swing kind, the contact the aim and the launch.
    #[test]
    fn the_us_first_serve_set_up_plays_as_recorded() {
        let Some((mut want, got, rngs)) = us(Some(380), 240, 0) else { return };
        us_first_tick(&mut want);
        serve_to_contact("us", &want, &got, 0, (169, 215, 219));
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
        // M183a1: the longer recording of slot 5's generators (`us_rng_rec.py 5 5200`, vsyncs 7520..12719, taken in
        // PCSX2 v2.9.52; it equals `rng_s05.bin` where they overlap, but for three torn samples) when it's there
        let long = std::fs::read(hst_data::fixture::path("fixtures/m183a1/us-rng-long.bin")).ok();
        // B767: slot 5's generators over the change of ends and game 2's first serve (`us_rng_window.py 5 2000 …
        // 12550`, vsyncs 12551..14550; B767's journal)
        let ends = std::fs::read(hst_data::fixture::path("fixtures/b767/us-rng-game2.bin")).ok();
        let keep = if ends.is_some() { 6815 } else if long.is_some() { 5150 } else { 1500 };
        let Some((want, got, rngs)) = us(None, keep, toss_ai) else { return };
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
        // M183a1: the contact height read up, and the reaction pulling a deep player to 4 m behind the baseline: the
        // phases through point 4's point-over, and every player on to point 5's rally (but each point-over's last tick)
        assert!(compare("us", &want, &got) >= 12, "the phases through point 4's point-over");
        // M183a1: a computer player's follow-through plays on until its own stick breaks it off (its first turn then starts
        // from the stroke's motion): every player on through point 6's serve to its rally, where play's serve, a tick
        // short at 30-40 as counted then (B766: it wasn't), puts play a tick ahead (and each point-over's last tick)
        assert_eq!((219..4011).filter(|&k| ![1084, 2354, 3190, 3795].contains(&k)).find(|&k| ground(&want[k]) != ground(&got[k + 1])), None, "the players part before tick 4011");
        // B766: the serve set-ups at 30-30, 30-40 and advantage weren't a tick long; the recording's first sample of
        // each came before its first tick ran and was counted as one (`recorded_ticks`): the first game and the change
        // of ends phase for phase, and every player on to the second game's first rally (and each point-over's last
        // tick)
        assert!(compare("us", &want, &got) >= 25, "the phases through the change of ends");
        assert_eq!((219..6845).filter(|&k| ![1084, 2354, 3190, 3795, 4391, 5011, 5689, 6543].contains(&k)).find(|&k| ground(&want[k]) != ground(&got[k + 1])), None, "the players part before tick 6845");
        // B767: between ends a computer player makes only its state's entry (no serve routine, walk or re-roll draws):
        // game 2's first serve set-up and its fault phase for phase, and every player on to the fault's point-over
        // (80 ticks in the game, 38 in play)
        assert!(compare("us", &want, &got) >= 27, "the phases through game 2's first serve's fault");
        assert_eq!((219..6899).filter(|&k| ![1084, 2354, 3190, 3795, 4391, 5011, 5689, 6543].contains(&k)).find(|&k| ground(&want[k]) != ground(&got[k + 1])), None, "the players part before tick 6899");
        // B769: the umpire's "Fault" is voiced, and the point-over waits for it: the fault's point-over phase for phase
        // (80 ticks; a draw at the serve's contact is still missing, B771)
        assert!(compare("us", &want, &got) >= 28, "the phases through game 2's first serve's point-over");
        // B771: the serving computer player hears its own serve's hit message (its formation repick, a moved spot
        // rolling the row's centre rate): game 2's second serve's set-up, and every player on to tick 7387 (and each
        // point-over's last tick, the fault's at 6941 too)
        assert!(compare("us", &want, &got) >= 29, "the phases through game 2's second serve's set-up");
        assert_eq!((219..7387).filter(|&k| ![1084, 2354, 3190, 3795, 4391, 5011, 5689, 6543, 6941].contains(&k)).find(|&k| ground(&want[k]) != ground(&got[k + 1])), None, "the players part before tick 7387");
        if let Some(d) = &ends {
            // the AI generator on the recording's draws from point 10's point-over through the change of ends and game
            // 2's serve set-up to the press, but at the two phase changes' ticks (as at every point's)
            let size = 4 + 8 + 4 * 0x9d0;
            let rec: Vec<hst_sim::rng::Mt> = d.chunks_exact(size).map(|s| hst_sim::rng::Mt::from_ram(&s[12 + 0x9d0..])).collect();
            let lost: Vec<usize> = (6500..6813).filter(|&k| !rec.contains(&rngs[k + 1].ai)).collect();
            assert_eq!(lost, [6544, 6625], "play's AI generator leaves the recording between ends");
            // B784: and `rand()` on the recording's through the change of ends (its start redraws the effects seed
            // before the sound manager's reseed)
            let rand: Vec<u64> = d.chunks_exact(size).map(|s| u64::from_le_bytes(s[4..12].try_into().unwrap())).collect();
            let lost: Vec<usize> = (6500..6813).filter(|&k| !rand.contains(&rngs[k + 1].rand.0)).collect();
            assert_eq!(lost, [6543, 6624], "play's rand() leaves the recording between ends (but the phase changes' ticks)");
        }
        if std::env::var_os("HST_M183A1_OFF").is_some() {
            let off: Vec<usize> = (219..want.len().min(got.len() - 1)).filter(|&k| ground(&want[k]) != ground(&got[k + 1])).take(40).collect();
            eprintln!("us players off at {off:?}");
        }
        first_point_part("us", &want, &got, 219);
        part_from("us", &want, &got);
        shot_probe("us", "fixtures/match_s05.bin", &want);
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
                // `HST_M183A1_WHO=n`: player n's position (default player 0)
                let who = std::env::var("HST_M183A1_WHO").ok().and_then(|v| v.parse().ok()).unwrap_or(0usize);
                eprintln!("us dump {k}: want {} {:?} {:?} | got {} {:?} {:?}", want[k].0, want[k].1[who], want[k].2, got[k + 1].0, got[k + 1].1[who], got[k + 1].2);
            }
        }
        // the generators beside slot 5's recorded ones (`fixtures/p3b/rng_s05.bin`, vsyncs 7533..9032; the serve
        // set-up starts at 7566): each played tick's rand / shared / AI generator, at the recorded vsync it equals
        if let Some(d) = long.or_else(|| hst_data::fixture::read("fixtures/p3b/rng_s05.bin").ok()) {
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
            // M183a1: a computer player's follow-through held until its own stick breaks it off: on through point 6's
            // serve contact (3797), past which play's serve, a tick short, runs a tick ahead
            let lost: Vec<usize> = (2..last - 6).filter(|&k| !rec.iter().any(|x| x.3 == rngs[k + 1].ai)).take_while(|&k| k < 3900).collect();
            // B768: the server's serve spot and wait drawn in its serve state's entry tick, beside the entries: the
            // set-ups' second ticks (1086, 2356, 3192, 3797) on the recording's draws too, only the serves' contacts off
            let expect: Vec<usize> = [218, 1269, 2578, 3484].into_iter().filter(|&k| k < last - 6).collect();
            assert_eq!(lost, expect, "play's AI generator is off the recording's draws but at the serves' contacts and point-overs");
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

    /// M183a1 probe: at each point's serve set-up (the recording's phase-2 starts through tick `HST_M183A1_KEEP`), how
    /// many `rand()` outputs the game's shared reseed sits from play's `rand()` at that tick (`--nocapture`).
    #[test]
    #[ignore]
    fn eu_point_reseeds() {
        use hst_sim::rng::Mt;
        // the whole match when it's there (36000 frames), else its first 7000
        let full = std::fs::read(hst_data::fixture::path("fixtures/m183a1/eu-rng-full.bin"));
        let Ok(d) = full.or_else(|_| std::fs::read(hst_data::fixture::path("fixtures/m183a1/eu-rng-long.bin"))) else { return };
        let Ok(m) = std::fs::read(hst_data::fixture::path("fixtures/m169/eu-bot-match.bin")) else { return };
        let frames = hst_sim::replay::frames_live(&m);
        let start = frames.iter().find(|f| f.gm()[0x55] == 2).unwrap().vsync() as i64;
        let rec: Vec<(i64, Mt)> = d.as_chunks::<{ 4 + 3 * 0x9d0 }>().0.iter().map(|s| (u32::from_le_bytes(s[..4].try_into().unwrap()) as i64 - start, Mt::from_ram(&s[4..]))).collect();
        let keep = std::env::var("HST_M183A1_KEEP").ok().and_then(|v| v.parse().ok()).unwrap_or(6200);
        let Some((want, _, rngs)) = european_rngs(Some(keep), 0, keep) else { return };
        let vsyncs = recorded_vsyncs(&m);
        let mut at = 0;
        for (ph, len, _) in phases(&want) {
            let k = at;
            at += len as usize;
            if ph != 2 || k + 3 >= rngs.len() {
                continue;
            }
            let Some(r) = vsyncs.get(k + 3).copied().flatten().and_then(|v| rec.iter().find(|x| x.0 == v)) else { continue };
            let mut g = r.1.clone();
            let target = [g.next(), g.next()];
            let mut st = rand_back(rngs[k].rand.0, 20_000);
            let found = (0..40_000i64).find(|_| {
                let mut r = hst_sim::rng::Rand(st);
                let mut mt = Mt::new(r.next());
                st = r.0;
                let outs: Vec<u32> = (0..300).map(|_| mt.next()).collect();
                outs.windows(2).any(|w| w == target)
            });
            eprintln!("eu point at tick {k}: the game's shared seed is play's rand() output {:?} (from play's state after that tick)", found.map(|n| n - 20_000));
        }
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

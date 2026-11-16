//! The European program at 50 Hz (`hst_sim::build::Build::Europe`), against M177's recordings of a CPU-only
//! doubles match on the European disc (`fixtures/m169/`, Alpine Heights, Yelena/Yuki vs Yuki/Yelena; not in git) and
//! that disc (`fixtures/discs/SCES-54535.iso`). Skipped without them, even under `HST_REQUIRE_DISC`: neither is the
//! user's disc.

use hst_data::fixture;
use hst_data::iso::Iso;
use hst_sim::build::Build;

const DISC: &str = "fixtures/discs/SCES-54535.iso";
/// The European program's match globals (its data sits 0x14e80 above the US program's).
const GLOBALS: usize = 0x437e00;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn disc() -> Option<Iso> {
    if !fixture::path(DISC).exists() {
        eprintln!("skipped: no {DISC}");
        return None;
    }
    Some(Iso::open(fixture::path(DISC)).unwrap())
}

fn fixture(name: &str) -> Option<Vec<u8>> {
    let path = fixture::path(format!("fixtures/m169/{name}"));
    let data = std::fs::read(&path).ok();
    if data.is_none() {
        eprintln!("skipped: no {}", path.display());
    }
    data
}

/// The four players' characters (+0x12bc) in a RAM image of the European program.
fn lineup(ram: &[u8]) -> [usize; 4] {
    let u = |a: usize| u32::from_le_bytes(ram[a & 0x1ff_ffff..][..4].try_into().unwrap()) as usize;
    [0, 1, 2, 3].map(|p| u(u(u(GLOBALS) + 0xa8 + 4 * p) + 0x12bc))
}

/// Every player motion's length in frames as the European program loads it (`Build::motion_rate`,
/// `Build::clip_length`) equals the length its motion player held, bit for bit, through the start of the match
/// (`eu-bot-anim.bin`, `tools/record_anim.py`: per player the motion number and its clip's length each frame; the
/// lineup from the later state's RAM, as the start state is taken before the players load): the swings and serve
/// swings keep their US frame counts, everything else (the whiffs too) is 5/6 as long, nudged when not whole.
#[test]
fn motion_lengths_match_the_european_program() {
    let (Some(mut iso), Some(anim), Some(ram)) = (disc(), fixture("eu-bot-anim.bin"), fixture("eu-bot-late_ee.bin")) else { return };
    const S: usize = 4 + 0x100 + 4 * 0x484;
    let chars = lineup(&ram);
    // (motion, clip pointer, clip length) of player p at sample k
    let at = |k: usize, p: usize| {
        let b = &anim[k * S + 0x104 + p * 0x484..][..0x484];
        (i(b, 0x420), i(b, 0x424), f(b, 0x480).to_bits())
    };
    let mut seen = std::collections::BTreeMap::new();
    for k in 1..anim.len() / S - 1 {
        for (p, &c) in chars.iter().enumerate() {
            // a motion held over three samples (the clip and its length are read a moment apart from the number)
            let (a, b, n) = (at(k - 1, p), at(k, p), at(k + 1, p));
            if a == b && b == n && b.1 != 0 {
                seen.entry((c, b.0)).or_insert_with(Vec::new).push(b.2);
            }
        }
    }
    let mut cache: std::collections::HashMap<usize, Vec<u8>> = std::collections::HashMap::new();
    let (mut checked, mut swings) = (0, 0);
    let mut bad = Vec::new();
    for ((c, motion), lengths) in &seen {
        // from 48 the motion player's numbers are the team reactions' (`motion::id::TEAM`), not the dummy clips
        if !(0..48).contains(motion) {
            continue;
        }
        let name = hst_data::ani::motion_name(*motion as usize, *c).unwrap();
        let xb = cache.entry(*c).or_insert_with(|| iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap());
        let arc = hst_data::xb::Archive::parse(xb).unwrap();
        let stem = format!("{}.ani2", name.to_ascii_lowercase());
        let Some(e) = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&stem)) else {
            bad.push(format!("character {c} motion {motion}: no {name}"));
            continue;
        };
        let a = hst_data::ani::parse(&arc.read(e).unwrap()).unwrap();
        let rate = Build::Europe.motion_rate(a.ticks_per_frame, Some(*motion as usize));
        let want = Build::Europe.clip_length(a.end_tick() as f32, rate as f32);
        for &l in lengths.iter().collect::<std::collections::BTreeSet<_>>() {
            if l != want.to_bits() {
                bad.push(format!("character {c} motion {motion} ({name}): game {} port {want}", f32::from_bits(l)));
            }
        }
        checked += 1;
        swings += (rate == a.ticks_per_frame) as usize;
    }
    eprintln!("{checked} (character, motion) lengths checked, {swings} at the US rate");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert!(checked > 60 && swings > 25, "{checked} {swings}");
}

/// The live ball of the European bot match (`eu-bot-match.bin`, `tools/record_p2m2.py`), each flying frame loaded
/// and stepped once under the European build against the court's world from the European disc: every frame bit for
/// bit, bounces and net touches included (gravity 9.8 / 2500 per frame²; the 0.003 term of the response unchanged).
#[test]
fn live_ball_flies_as_the_european_program() {
    use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
    use hst_sim::replay::frames_live;
    let (Some(mut iso), Some(data)) = (disc(), fixture("eu-bot-match.bin")) else { return };
    let frames = frames_live(&data);
    let court = frames.last().unwrap().global(0x422f90) as usize;
    let game_bin = iso.read("ZZBIN/GAME.BIN").unwrap();
    let materials = hst_sim::court::materials(&hst_data::exe::Game::new(&iso.read("SYSTEM.CNF").unwrap(), &game_bin).unwrap());
    let world = hst_sim::court::world(&mut iso, court as u32).unwrap();
    let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
    let rows = |b: &[u8], o: usize| -> [[f32; 4]; 4] { std::array::from_fn(|r| std::array::from_fn(|k| f(b, o + 16 * r + 4 * k))) };
    let (mut exact, mut touches, mut bad) = (0, 0, Vec::new());
    for w in frames.windows(2) {
        let (a, b) = (w[0].live_ball(), w[1].live_ball());
        let carried = |o: &[u8]| v3(o, 0x130) == [0.0; 3];
        if w[1].vsync() != w[0].vsync() + 1 || i(b, 0xac) != i(a, 0xac) + 1 || a[0xa4] > 1 || b[0xa4] > 1 || carried(a) || carried(b) {
            continue;
        }
        let ball = Ball { pos: v3(a, 0xe0), vel: v3(a, 0x130), spin: f(a, 0x1a4) };
        let mut fl = Flight::new(ball, rows(a, 0x160), rows(a, 0x1c0));
        fl.build = Build::Europe;
        fl.frame = i(a, 0xac);
        fl.bounces = i(a, 0x224);
        fl.contacts = i(a, 0x228);
        fl.special_contacts = i(a, 0x22c);
        fl.rolling = a[0xa4] == 2;
        fl.slide = f(a, 0x200);
        let shot = Shot {
            params: Params::default(),
            curve: f(a, 0x254),
            bend: f(a, 0x250),
            side: v3(a, 0x90),
            curve_frames: i(a, 0x260),
            wind: v3(a, 0x240),
            first_bounce_spin: f(a, 0x1a8),
            first_bounce_restitution: f(a, 0x1ac),
            class: a[0x58],
            kind: i(a, 0x5c),
            bounce_turn: f(a, 0x1b0),
        };
        fl.step_world(&shot, &COURTS[court], &world, &materials);
        let want = [v3(b, 0xe0), v3(b, 0x130)].concat();
        let got = [fl.ball.pos, fl.ball.vel].concat();
        touches += (i(b, 0x224) != i(a, 0x224) || i(b, 0x22c) != i(a, 0x22c)) as usize;
        if (0..6).all(|k| got[k].to_bits() == want[k].to_bits()) {
            exact += 1;
        } else {
            bad.push(format!("vsync {} material {}: got {got:?} want {want:?}", w[1].vsync(), b[0x220]));
        }
    }
    eprintln!("court {court}: {exact} frames bit-exact, {touches} contact frames");
    assert!(bad.is_empty(), "{} frames diverged:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    assert!(exact > 8000 && touches > 100, "{exact} {touches}");
}

fn p_u8(fr: hst_sim::replay::Frame, p: usize, off: usize) -> u8 {
    fr.player_f32(p, off & !3).to_bits().to_le_bytes()[off & 3]
}
fn p_i32(fr: hst_sim::replay::Frame, p: usize, off: usize) -> i32 {
    fr.player_f32(p, off).to_bits() as i32
}
fn p_v3(fr: hst_sim::replay::Frame, p: usize, off: usize) -> [f32; 3] {
    [0, 4, 8].map(|o| fr.player_f32(p, off + o))
}

/// Every toss of the European bot match leaves the hand at the European toss gravity's speed (0.9 × 9.8 / 2500 per
/// frame², bits 3b673604): the recorded hand and apex give the recorded launch velocity bit for bit.
#[test]
fn tosses_rise_at_the_european_gravity() {
    use hst_sim::replay::frames_live;
    let Some(data) = fixture("eu-bot-match.bin") else { return };
    let frames = frames_live(&data);
    let mut tosses = 0;
    for w in frames.windows(2) {
        let (a, fr) = (w[0], w[1]);
        for p in 0..4 {
            if p_u8(fr, p, 0x3ec0) == 1 && p_u8(a, p, 0x3ec0) == 0 {
                let hand = p_v3(fr, p, 0x3eb0);
                let apex = p_v3(fr, p, 0x3e90);
                let vel = hst_sim::serve::toss_velocity_in(hand, apex, Build::Europe.toss_gravity());
                let want = [0, 4, 8].map(|o| f(fr.live_ball(), 0x130 + o));
                assert_eq!(vel.map(f32::to_bits), want.map(f32::to_bits), "vsync {} p{p}: {vel:?} vs {want:?}", fr.vsync());
                tosses += 1;
            }
        }
    }
    eprintln!("{tosses} tosses bit-exact");
    assert!(tosses > 40, "{tosses}");
}

/// The CPU servers' walk along the baseline in the European bot match: each frame a server moves before its toss
/// (play state 1), the step is the European walk (bits 3d23d69f) toward where it went, clamped as in the US
/// (`player::serve_walk_in`), bit for bit.
#[test]
fn cpu_servers_walk_the_european_step() {
    use hst_sim::player::serve_walk_in;
    use hst_sim::replay::frames_live;
    let Some(data) = fixture("eu-bot-match.bin") else { return };
    let frames = frames_live(&data);
    let (mut steps, mut bad) = (0, Vec::new());
    for w in frames.windows(2) {
        let (a, fr) = (w[0], w[1]);
        if fr.vsync() != a.vsync() + 1 {
            continue;
        }
        for p in 0..4 {
            let pre = |x: hst_sim::replay::Frame| p_u8(x, p, 0x3fa4) == 1 && p_u8(x, p, 0x3fa6) <= 1;
            let (x, want) = (a.player_f32(p, 0x3d70), fr.player_f32(p, 0x3d70));
            if !pre(a) || !pre(fr) || x == want {
                continue;
            }
            let end = if a.player_f32(p, 0x3d78) < 0.0 { 1.0 } else { -1.0 };
            let dir = [if want < x { -1.0 } else { 1.0 }, 0.0];
            let got = serve_walk_in(x, dir, end, fr.global(0x423050), true, 1.0, Build::Europe).0;
            steps += 1;
            if got.to_bits() != want.to_bits() {
                bad.push(format!("vsync {} p{p}: {x} -> {got} vs {want}", fr.vsync()));
            }
        }
    }
    eprintln!("{steps} walk steps");
    assert!(bad.is_empty(), "{} off:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    assert!(steps > 100, "{steps}");
}

/// Every running frame of the European bot match moves at the European run speed (bits 3db03740 a frame, the US
/// formula around it: `player::run_speed_in`), from each player's stats in the European program's RAM, bit for bit.
#[test]
fn players_run_at_the_european_speed() {
    use hst_sim::player::{Stats, run_speed_in, run_velocity};
    use hst_sim::replay::frames_live;
    let (Some(data), Some(ram)) = (fixture("eu-bot-match.bin"), fixture("eu-bot-late_ee.bin")) else { return };
    let ru = |a: usize| u32::from_le_bytes(ram[a & 0x1ff_ffff..][..4].try_into().unwrap()) as usize;
    let stats: Vec<Stats> = (0..4)
        .map(|p| {
            let pl = ru(ru(GLOBALS) + 0xa8 + 4 * p);
            let i = |o: usize| ru(pl + o) as i32;
            Stats { speed: f(&ram, (pl + 0x1374) & 0x1ff_ffff), agility: i(0x1388), stamina: i(0x1378), dive: i(0x137c), backhand: i(0x1380), smash: i(0x1384) }
        })
        .collect();
    let frames = frames_live(&data);
    let (mut speeds, mut bad) = (0, Vec::new());
    for fr in &frames {
        for (p, s) in stats.iter().enumerate() {
            if p_u8(*fr, p, 0x3fa5) != 1 {
                continue;
            }
            let fwd = if fr.player_f32(p, 0x3d78) < 0.0 { 1.0 } else { -1.0 };
            let target = p_v3(*fr, p, 0x3dc0);
            if target == [0.0, 0.0, fwd] || target == [-0.0, -0.0, -fwd] {
                continue;
            }
            let (run, stamina) = (p_i32(*fr, p, 0x3dfc), p_i32(*fr, p, 0x3df4));
            let v = run_velocity(target, run_speed_in(s, run, stamina, 100, Build::Europe));
            let got = p_v3(*fr, p, 0x3e00);
            speeds += 1;
            if v.map(f32::to_bits) != got.map(f32::to_bits) {
                bad.push(format!("vsync {} p{p} run {run} stamina {stamina}: {v:?} vs {got:?}", fr.vsync()));
            }
        }
    }
    eprintln!("{speeds} run frames");
    assert!(bad.is_empty(), "{} off:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    assert!(speeds > 5000, "{speeds}");
}

fn tick(f: hst_sim::replay::Frame) -> u32 {
    u32::from_le_bytes(f.gm()[0x58..0x5c].try_into().unwrap())
}

/// The score as the recording holds it (the European program's globals, recorded at the US layout).
fn score_at(f: hst_sim::replay::Frame) -> hst_sim::score::Score {
    let (points, games, sets) = f.score();
    let u = |a| f.global_u8(a) != 0;
    hst_sim::score::Score {
        points,
        games,
        sets,
        deuce: f.rally_u8(0x316620) != 0,
        advantage: f.rally_u8(0x316628) != 0,
        tiebreak: f.rally_u8(0x31661a) != 0,
        tiebreak_count: f.rally(0x31661c),
        rotation: f.global(0x4230b0),
        swapped: u(0x4230b4),
        game_changed: u(0x4230ac),
        games_played: f.global(0x4230c0),
        match_over: u(0x4230be),
        server: f.global(0x42304c),
        side: f.global(0x423050),
        receiver: f.global(0x423054),
        ..hst_sim::score::Score::new()
    }
}

/// The rally block as `Rally` models it.
fn rally_block(f: hst_sim::replay::Frame) -> hst_sim::judge::Rally {
    use hst_sim::judge::{Call, Rally};
    let call = |v: u8| [Call::None, Call::In, Call::Out, Call::Net, Call::NetIn, Call::NetOut][v as usize];
    let r = |a| f.rally(a);
    let u = |a| f.rally_u8(a) != 0;
    Rally {
        call: call(f.rally_u8(0x3165f0)),
        shots: r(0x3165f4),
        hitter: r(0x3165f8),
        server: r(0x3165fc),
        illegal_hit: u(0x316604),
        faults: r(0x316608),
        let_: u(0x31660c),
        serve_call: call(f.rally_u8(0x316610)),
        checked_shots: r(0x316614),
        return_bounced: u(0x316618),
        hit_on_serve: u(0x316629),
        lose: u(0x31662a),
        serve_bounced: u(0x31662b),
    }
}

/// The point-over phase entered at `frames[0]` under `build`'s flow, fed the recorded score and rally block at the
/// decision: faults, points and games must match every game tick, and the next phase must be asked for on the
/// game's tick. `settle`: the tick the umpire's call line ends (`None`: the countdown settles the call).
fn post_point(frames: &[hst_sim::replay::Frame], t: &hst_data::exe::ScoreboardTiming, build: Build, settle: Option<u32>) -> Result<u32, String> {
    use hst_sim::flow::{Next, PostPoint};
    use hst_sim::score::Event;
    let f = frames[0];
    let mut score = score_at(f);
    let mut rally = rally_block(f);
    let body = Some(f.global(0x42305c)).filter(|&p| p >= 0);
    let event = match f.global(0x4230b8) {
        0 if score.tiebreak => Some(Event::TiebreakPoint),
        0 => Some(Event::Point),
        1 => Some(Event::Game),
        2 => Some(Event::Set),
        _ => None,
    };
    let mut pp = PostPoint::called(event, rally.judge(body).call, t).in_build(build);
    let mut asked = None;
    for &s in frames {
        while pp.tick < tick(s) {
            let idle = settle.is_some_and(|v| pp.tick + 1 >= v);
            if let Some(n) = pp.step_with(&mut score, &mut rally, t, idle, false) {
                asked.get_or_insert((n, pp.tick));
            }
        }
        if s.gm()[0x56] != 0xff {
            let want = match s.gm()[0x56] {
                1 => Next::ChangeEnds,
                2 => Next::Serve,
                _ => Next::MatchOver,
            };
            return if asked == Some((want, tick(s))) { Ok(tick(s)) } else { Err(format!("{event:?}: asked {asked:?}, game {want:?} at {}", tick(s))) };
        }
        if asked.is_some() {
            return Err(format!("{event:?}: port asked early ({asked:?}), game at tick {}", tick(s)));
        }
        let want = (s.rally(0x316608), s.score().0, s.score().1);
        if (rally.faults, score.points, score.games) != want {
            return Err(format!("{event:?} tick {}: port {:?}, game {want:?}", tick(s), (rally.faults, score.points, score.games)));
        }
    }
    Err("recording ends mid-phase".into())
}

/// Every point-over phase of the European bot match (the match-over point left out) under `build`: a called one
/// must match for one tick where her call line ends (searched from 1 to the call's countdown) or on the countdown
/// alone (pushed as 0). Returns (phases, failures, called phases' line ends by call).
fn post_points(frames: &[hst_sim::replay::Frame], t: &hst_data::exe::ScoreboardTiming, build: Build) -> (usize, Vec<String>, Vec<(u8, u32)>) {
    let (mut checked, mut bad, mut lines) = (0, Vec::new(), Vec::new());
    for k in 1..frames.len() {
        let (a, f) = (frames[k - 1], frames[k]);
        if !(f.gm()[0x55] == 4 && a.gm()[0x55] != 4) || score_at(f).match_over {
            continue;
        }
        let call = rally_block(f).judge(Some(f.global(0x42305c)).filter(|&p| p >= 0)).call;
        checked += 1;
        if call == 0 || call == 6 {
            if let Err(e) = post_point(&frames[k..], t, build, None) {
                bad.push(format!("vsync {}: {e}", f.vsync()));
            }
            continue;
        }
        let line = (1..=t.call_wait[call as usize] as u32).find(|&v| post_point(&frames[k..], t, build, Some(v)).is_ok());
        match line.or_else(|| post_point(&frames[k..], t, build, None).is_ok().then_some(0)) {
            Some(v) => lines.push((call, v)),
            None => bad.push(format!("vsync {}: call {call}: {}", f.vsync(), post_point(&frames[k..], t, build, None).unwrap_err())),
        }
    }
    (checked, bad, lines)
}

/// The point-over and change-ends phases of the European bot match run on the European flow (`PostPoint::in_build`,
/// `flow::change_ends`): the scoreboard pauses 25 ticks (not 30), a score stays up its hold × 50 ticks, the call
/// sprite's hold and fade stay the US ticks while her line (5/6 as many ticks) settles it sooner, the change-ends cut is 66 ticks; the scoreboard's waits and show steps are the
/// US table's, read at the European GAME.BIN's offsets. Every point, game and change of ends asks for the next
/// phase on the game's own tick (134 ticks a point, 255 a game, 67 a change of ends). The US flow misses every one.
#[test]
fn match_flow_runs_on_the_european_timing() {
    use hst_sim::replay::frames_live;
    let (Some(mut iso), Some(data)) = (disc(), fixture("eu-bot-match.bin")) else { return };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    // the match's umpire and voice from its start state (the US menu bytes, at the European boot data's shift)
    let ram = fixture("eu-bot-start_ee.bin").unwrap();
    let (voice, umpire) = (ram[0x2ef7df + 0x14d00], ram[0x2ef7de + 0x14d00]);
    eprintln!("umpire {umpire} voice {voice}");
    let t = hst_data::exe::Game::new(&cnf, &bin).unwrap().scoreboard_timing(voice, umpire);
    let frames = frames_live(&data);
    let mut changes = 0;
    for k in 1..frames.len() {
        let (a, f) = (frames[k - 1], frames[k]);
        if f.gm()[0x55] == 1 && a.gm()[0x55] != 1 {
            let end = frames[k..].iter().find(|s| s.gm()[0x56] != 0xff).unwrap();
            assert_eq!((end.gm()[0x56], tick(*end)), (2, hst_sim::flow::change_ends(Build::Europe) + 1), "vsync {}: change ends", f.vsync());
            changes += 1;
        }
    }
    let (checked, bad, lines) = post_points(&frames, &t, Build::Europe);
    eprintln!("{checked} point-over phases (call line ends {lines:?}), {changes} change-ends phases");
    assert!(bad.is_empty(), "{} off:\n{}", bad.len(), bad.join("\n"));
    assert!(checked > 30 && !lines.is_empty() && changes >= 2, "{checked} {changes}");
    // umpire 0 a's line calls (out, fault, double fault, let, out after net) are 28, 29, 47, 29, 29 frames of audio
    // at 60 Hz: 5/6 as many ticks here (integer), heard stopping `VOICE_LAG` ticks after, a tick sooner now and then
    assert_eq!(umpire, 0, "the call clips below are umpire 0's");
    let clip = [0, 28, 29, 47, 29, 29].map(|n: u32| n * 50 / 60);
    let lag = hst_sim::umpire::VOICE_LAG;
    let after = |&(c, v): &(u8, u32)| v - clip[c as usize];
    assert!(lines.iter().all(|l| after(l) == lag || after(l) == lag - 1), "{lines:?}");
    assert!(lines.iter().filter(|l| after(l) == lag).count() * 2 > lines.len(), "{lines:?}");
    let (_, us_bad, _) = post_points(&frames, &t, Build::Us);
    assert!(us_bad.len() * 2 > checked, "the US flow matched {} of {checked}", checked - us_bad.len());
}


//! M139 (ours, by the user's request; the original never rumbles in a match): each human's own controller rumbles on
//! their hits (by the ball's launch speed, a kick more on the sweet spot; a short tick on a mis-timed or framed one),
//! when the ball hits them, and a longer buzz when their side wins a point, more for a game or set, most for the
//! match. Strengths and lengths are by feel.
//!
//! Presentation only: it watches the match state once a display frame and never writes to it, so the sim, its RNG and
//! the timing are untouched. Each event is taken once (a rollback's re-simulated ticks don't buzz again). No rumble for
//! a computer player, another machine's player online, an on-screen pad, an instant replay or a played-back match.
//! Settings → Display → Controller Vibration (`ON`, kept by the menus) switches it off.

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::controls::PadSlots;
use super::netplay::Netplay;
use super::tick_input::Replay;
use super::{Game, Pads, Phase};

/// The Controller Vibration setting, as the main menu (and the pause menu's Settings) last had it.
pub static ON: AtomicBool = AtomicBool::new(true);

/// One rumble: the big (low, thumping) and small (high, buzzing) motors' strengths (0..1) and its length (seconds).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Buzz {
    strong: f32,
    weak: f32,
    secs: f32,
}

/// The ball's launch speed (m per tick) a hit's rumble starts from and the one it peaks at: a soft lob or drop shot
/// is about the first, a full serve or smash the second (a bot match's rallies run 0.27..0.75, half above 0.55).
const SLOW: f32 = 0.3;
const FAST: f32 = 0.75;

/// A clean hit: the faster the ball leaves, the harder and longer the thump; the sweet spot adds a little.
fn hit(speed: f32, sweet: bool) -> Buzz {
    let t = ((speed - SLOW) / (FAST - SLOW)).clamp(0.0, 1.0);
    let t = if sweet { (t + 0.15).min(1.0) } else { t };
    Buzz { strong: t * t, weak: 0.35 + 0.5 * t, secs: 0.07 + 0.11 * t }
}

/// A mis-timed (late or early by two frames or more) or framed hit: a short, light tick.
const MISHIT: Buzz = Buzz { strong: 0.0, weak: 0.3, secs: 0.05 };
/// The ball hitting the player.
const STRUCK: Buzz = Buzz { strong: 0.8, weak: 0.6, secs: 0.25 };
/// The player's side won the point, a game or set, the match.
const POINT: Buzz = Buzz { strong: 0.15, weak: 0.5, secs: 0.35 };
const GAME: Buzz = Buzz { strong: 0.45, weak: 0.7, secs: 0.6 };
const MATCH: Buzz = Buzz { strong: 0.8, weak: 1.0, secs: 1.2 };

/// What has already buzzed: the most shots seen this rally, whether the body hit has, and the score the last point's
/// result buzzed at (an instant replay comes back to the same point's end: it doesn't buzz twice).
#[derive(Default)]
struct Seen {
    shots: i32,
    struck: bool,
    won: Option<hst_sim::score::Score>,
}

/// The rumbles due for what changed since `seen`, each for a player: their hit, the ball hitting them, their side
/// winning the point (or game, set, match).
fn due(g: &Game, seen: &mut Seen) -> Vec<(usize, Buzz)> {
    let mut out = Vec::new();
    // a new rally (the serve set up again) starts the count over
    if g.shots == 0 {
        seen.shots = 0;
    }
    if g.shots > seen.shots {
        seen.shots = g.shots;
        if let Ok(who) = usize::try_from(g.last_hitter) {
            let ball = g.hit_flight.ball.vel;
            let speed = (ball[0] * ball[0] + ball[1] * ball[1] + ball[2] * ball[2]).sqrt();
            let offset = g.instant.timing.get(who).copied().unwrap_or(0);
            let serve = g.shots == 1;
            let buzz = if g.players.get(who).is_some_and(|p| p.framed) || (!serve && offset.abs() >= 2) { MISHIT } else { hit(speed, offset == 0) };
            out.push((who, buzz));
        }
    }
    match g.body_hit {
        None => seen.struck = false,
        Some(i) if !seen.struck => {
            seen.struck = true;
            out.extend(usize::try_from(i).ok().map(|i| (i, STRUCK)));
        }
        _ => {}
    }
    if let Some(p) = g.post.as_ref().filter(|p| g.phase == Phase::Post && p.event.is_some() && (p.reacted || p.calling_match) && seen.won.as_ref() != Some(&g.score)) {
        seen.won = Some(g.score.clone());
        let buzz = if g.score.match_over {
            MATCH
        } else if matches!(p.event, Some(hst_sim::score::Event::Game | hst_sim::score::Event::Set)) {
            GAME
        } else {
            POINT
        };
        out.extend((0..g.players.len()).filter(|&i| i as i32 & 1 == g.post_winner).map(|i| (i, buzz)));
    }
    out
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, rumble);
}

fn rumble(
    g: Option<Res<Game>>,
    pads: Res<Pads>,
    slots: Res<PadSlots>,
    net: Option<Res<Netplay>>,
    replay: Option<Res<Replay>>,
    names: Query<Option<&Name>, With<Gamepad>>,
    mut out: MessageWriter<GamepadRumbleRequest>,
    mut seen: Local<Seen>,
) {
    let Some(g) = g else { return };
    let due = due(&g, &mut seen);
    if due.is_empty() || !ON.load(Ordering::Relaxed) || g.instant.replaying || replay.is_some() {
        return;
    }
    let n = g.players.len();
    for (i, b) in due {
        // a human of this machine's, on a real controller
        let Some(slot) = pads.slot_of(i, n).filter(|s| net.as_ref().is_none_or(|np| np.lock.mine.contains(s))) else { continue };
        let Some(pad) = slots.0[slot].filter(|&e| names.get(e).is_ok_and(|n| !n.is_some_and(|n| super::emote::is_touch(n)))) else { continue };
        out.write(GamepadRumbleRequest::Add {
            gamepad: pad,
            intensity: GamepadRumbleIntensity { strong_motor: b.strong, weak_motor: b.weak },
            duration: Duration::from_secs_f32(b.secs),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Harder hits rumble harder and longer; the sweet spot adds; every rumble stays in range; a mis-hit is the
    /// lightest, the match the longest.
    #[test]
    fn strengths_by_feel() {
        let (soft, hard) = (hit(SLOW, false), hit(FAST, false));
        assert!(soft.strong < hard.strong && soft.weak < hard.weak && soft.secs < hard.secs);
        assert!(hit(0.5, true).strong > hit(0.5, false).strong);
        for b in [soft, hard, hit(0.0, true), hit(5.0, true), MISHIT, STRUCK, POINT, GAME, MATCH] {
            assert!((0.0..=1.0).contains(&b.strong) && (0.0..=1.0).contains(&b.weak) && b.secs > 0.0 && b.secs <= 1.5, "{b:?}");
        }
        assert!(MISHIT.secs < soft.secs && POINT.secs < GAME.secs && GAME.secs < MATCH.secs);
    }
}

#[cfg(test)]
mod in_match {
    use super::super::*;
    use super::{due, Seen, GAME, MISHIT, POINT};
    use crate::{Args, GameSpace};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A bot match's hits each buzz their hitter once (by launch speed), and won points buzz the winning side once
    /// each: the state `due` reads moves as it expects.
    #[test]
    fn match_events_buzz_once() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let mut seen = Seen::default();
        let (mut hits, mut wins, mut speeds) = (0, 0, Vec::new());
        for _ in 0..60 * 120 {
            let before = app.world().resource::<Game>().shots;
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            let out = due(g, &mut seen);
            if g.shots > before {
                let b = g.hit_flight.ball.vel;
                speeds.push((b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt());
                assert!(out.iter().any(|&(i, _)| i as i32 == g.last_hitter), "hit {hits}: no buzz for its hitter");
                hits += 1;
            }
            for (i, b) in out {
                if b == POINT || b == GAME {
                    assert_eq!(i as i32 & 1, g.post_winner, "a loser buzzed for the point");
                    wins += 1;
                }
                assert!(b == MISHIT || b.secs > 0.0);
            }
            // the same state again buzzes nothing
            assert!(due(g, &mut seen).is_empty());
        }
        speeds.sort_by(f32::total_cmp);
        eprintln!("{hits} hits, {wins} win buzzes; launch speeds {:?}", [0, speeds.len() / 4, speeds.len() / 2, speeds.len() * 3 / 4, speeds.len() - 1].map(|k| speeds[k]));
        assert!(hits > 10 && wins > 2, "{hits} hits, {wins} wins");
    }
}

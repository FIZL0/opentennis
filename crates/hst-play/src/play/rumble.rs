//! M172 (ours, by the user's request; the original never rumbles in a match, M138): Settings → Gameplay → Match Rumble
//! makes each human's own controller answer the ball:
//!
//! - a mistimed hit (the early or late balloon) or a framed one: a big, heavy rumble, the racket shaking in the hand;
//! - a sweet-spot hit (the sweet balloon): a short, crisp tick on the light motor only;
//! - a hard serve hitting the ground: a thump for the receiver (it lands at them), a softer one for the server;
//! - the ball hitting the player: a thump.
//!
//! A clean hit off the sweet spot (the note balloon), a dive, points and games stay quiet, so the three that matter
//! read clearly. Strengths and lengths are by feel. Pads only: the on-screen touch pad keeps its own press buzz (Settings
//! → Gameplay → Vibration) and gets none of these.
//!
//! Presentation only: it reads the match after each tick and never writes to it, so the sim, its RNG and the timing
//! are untouched. Each event is keyed by the match frame it happened on, so a rollback's re-simulated ticks don't
//! rumble again. Nothing for a computer player, another machine's player online, an instant replay or a played-back
//! match. Off by default (`ON`, kept by the menus).

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::controls::PadSlots;
use super::netplay::Netplay;
use super::tick_input::Replay;
use super::{Game, Pads};
use hst_sim::serve::Balloon;

/// Settings → Gameplay → Match Rumble, as the menus last had it (off until they say otherwise).
pub static ON: AtomicBool = AtomicBool::new(false);

/// One rumble: the big (low, thumping) and small (high, buzzing) motors' strengths (0..1) and its length (seconds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Buzz {
    pub strong: f32,
    pub weak: f32,
    pub secs: f32,
}

/// A mistimed or framed hit: both motors hard, long enough to feel the racket ring.
pub(super) const MISTIMED: Buzz = Buzz { strong: 1.0, weak: 0.6, secs: 0.3 };
/// A sweet-spot hit: the light motor alone, a few frames.
pub(super) const SWEET: Buzz = Buzz { strong: 0.0, weak: 0.75, secs: 0.06 };
/// A hard serve landing: on the receiver's pad, and the server's, softer.
pub(super) const THUMP: Buzz = Buzz { strong: 0.8, weak: 0.1, secs: 0.12 };
pub(super) const THUMP_FAR: Buzz = Buzz { strong: 0.35, weak: 0.0, secs: 0.08 };
/// The ball hitting the player.
pub(super) const STRUCK: Buzz = Buzz { strong: 0.7, weak: 0.4, secs: 0.2 };

/// A serve this fast or more (km/h, as the speed readout shows it) thumps when it lands: a well-struck power serve (a
/// bot match's strong tosses run 108..136, its weak ones 59..118); a placed, mistimed or second serve stays quiet.
pub(super) const HARD_SERVE: f32 = 125.0;

/// What happened, for the key that stops a re-simulated tick rumbling twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum What {
    Hit,
    Bounce,
    Struck,
}

/// The rumbles due for what changed since the last tick, each for a player.
#[derive(Default)]
pub(super) struct Watch {
    shots: i32,
    bounced: bool,
    struck: Option<i32>,
    /// The latest events' keys (match frame, what, whom): a rollback replays them, they don't rumble again.
    sent: VecDeque<(u32, What, usize)>,
}

impl Watch {
    /// The rumbles the tick just played calls for. Reads the match only; run between the hit (which sets the balloon
    /// and the framed flag) and the pop-ups (which take the framed flag and may drop the balloon).
    pub(super) fn due(&mut self, g: &Game) -> Vec<(usize, Buzz)> {
        let mut out = Vec::new();
        let frame = g.match_stats.frames;
        let mut push = |w: &mut Self, what: What, who: usize, b: Buzz| {
            if w.sent.contains(&(frame, what, who)) {
                return;
            }
            if w.sent.len() >= 16 {
                w.sent.pop_front();
            }
            w.sent.push_back((frame, what, who));
            out.push((who, b));
        };
        // a new point (or a rollback to before the last hit) starts the count over
        if g.shots < self.shots {
            self.shots = g.shots;
        }
        if g.shots == 0 {
            self.bounced = false;
        }
        if g.shots > self.shots {
            self.shots = g.shots;
            if let Some(who) = usize::try_from(g.last_hitter).ok().filter(|&i| i < g.players.len()) {
                let p = &g.players[who];
                // a balloon set by this hit is one tick old here (an older one is the last hit's)
                let fresh = p.balloon.filter(|&(_, age)| age <= 1).map(|(b, _)| b);
                let buzz = if p.framed || matches!(fresh, Some(Balloon::Turtle | Balloon::Bunny)) {
                    Some(MISTIMED)
                } else if fresh == Some(Balloon::Sweet) {
                    Some(SWEET)
                } else {
                    None
                };
                if let Some(b) = buzz {
                    push(self, What::Hit, who, b);
                }
            }
        }
        // the serve's first landing
        if g.shots == 1 && g.flight.bounces > 0 && !self.bounced {
            self.bounced = true;
            if g.build.kmh(g.hit_flight.ball.vel) >= HARD_SERVE {
                let n = g.players.len();
                if let Some(r) = usize::try_from(g.score.receiver).ok().filter(|&r| r < n) {
                    push(self, What::Bounce, r, THUMP);
                }
                if let Some(s) = usize::try_from(g.last_hitter).ok().filter(|&s| s < n) {
                    push(self, What::Bounce, s, THUMP_FAR);
                }
            }
        }
        if g.body_hit != self.struck {
            self.struck = g.body_hit;
            if let Some(i) = g.body_hit.and_then(|i| usize::try_from(i).ok()).filter(|&i| i < g.players.len()) {
                push(self, What::Struck, i, STRUCK);
            }
        }
        out
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(FixedUpdate, rumble.after(super::age_balloons).before(super::surprise::tick));
}

#[allow(clippy::too_many_arguments)]
fn rumble(
    g: Option<Res<Game>>,
    pads: Option<Res<Pads>>,
    slots: Option<Res<PadSlots>>,
    net: Option<Res<Netplay>>,
    replay: Option<Res<Replay>>,
    names: Query<Option<&Name>, With<Gamepad>>,
    out: Option<ResMut<Messages<GamepadRumbleRequest>>>,
    mut watch: Local<Watch>,
) {
    let Some(g) = g else { return };
    // watched even while off, so switching it on mid-match doesn't rumble for what's already past
    let due = watch.due(&g);
    let (Some(pads), Some(slots), Some(mut out)) = (pads, slots, out) else { return };
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
    use super::super::*;
    use super::{Buzz, Watch, HARD_SERVE, MISTIMED, STRUCK, SWEET, THUMP, THUMP_FAR};
    use crate::{Args, GameSpace};

    // The three the user asked for feel apart (checked when it builds): the mistimed hit the heaviest and longest, the
    // sweet spot short and on the light motor only, the serve's thump on the heavy motor.
    const _: () = assert!(MISTIMED.strong > THUMP.strong && MISTIMED.secs > STRUCK.secs && STRUCK.secs > THUMP.secs && THUMP.secs > SWEET.secs);
    const _: () = assert!(SWEET.strong == 0.0 && SWEET.weak > 0.5);
    const _: () = assert!(THUMP.strong > THUMP.weak && THUMP_FAR.strong < THUMP.strong);

    /// Every rumble's motors are in 0..1 and its length short.
    #[test]
    fn rumbles_in_range() {
        for b in [MISTIMED, SWEET, THUMP, THUMP_FAR, STRUCK] {
            assert!((0.0..=1.0).contains(&b.strong) && (0.0..=1.0).contains(&b.weak) && b.secs > 0.0 && b.secs <= 0.5, "{b:?}");
        }
    }

    /// A bot match (all players' events, as if each were a human): hits with an early/late balloon rumble big, sweet
    /// ones crisp, clean ones not at all; a hard serve's landing thumps its receiver and server once; nothing twice;
    /// replaying the same ticks (a rollback) rumbles nothing again.
    #[test]
    fn match_events_rumble_once() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let mut watch = Watch::default();
        let (mut big, mut crisp, mut quiet, mut thumps) = (0, 0, 0, 0);
        let mut serves = Vec::new();
        let mut rewind: Option<(Game, Vec<(usize, Buzz)>)> = None;
        for t in 0..60 * 150 {
            let before = app.world().resource::<Game>().clone();
            app.world_mut().run_schedule(FixedUpdate);
            // the match clock the keys read (`match_stats`' tick, not in this chain)
            app.world_mut().resource_mut::<Game>().match_stats.tick();
            let g = app.world().resource::<Game>();
            let out = watch.due(g);
            if g.shots > before.shots {
                let who = g.last_hitter as usize;
                let b = g.players[who].balloon.filter(|&(_, age)| age <= 1).map(|(b, _)| b);
                let mine: Vec<_> = out.iter().filter(|(i, _)| *i == who).map(|(_, b)| *b).collect();
                match b {
                    Some(hst_sim::serve::Balloon::Turtle | hst_sim::serve::Balloon::Bunny) => {
                        assert_eq!(mine, [MISTIMED], "tick {t}");
                        big += 1;
                    }
                    Some(hst_sim::serve::Balloon::Sweet) => {
                        assert_eq!(mine, [SWEET], "tick {t}");
                        crisp += 1;
                    }
                    _ if !g.players[who].framed => {
                        assert!(mine.is_empty(), "tick {t}: {mine:?} for {b:?}");
                        quiet += 1;
                    }
                    _ => {}
                }
                if g.shots == 1 {
                    serves.push((g.build.kmh(g.hit_flight.ball.vel), g.finish.strong));
                }
            }
            if g.shots == 1 && g.flight.bounces > 0 && before.flight.bounces == 0 && before.shots == 1 {
                let hard = g.build.kmh(g.hit_flight.ball.vel) >= HARD_SERVE;
                let got: Vec<_> = out.iter().filter(|(_, b)| *b == THUMP || *b == THUMP_FAR).collect();
                assert_eq!(got.len(), if hard { 2 } else { 0 }, "tick {t}: {got:?}");
                if hard {
                    assert!(got.iter().any(|&&(i, b)| b == THUMP && i as i32 == g.score.receiver), "tick {t}: {got:?}");
                    thumps += 1;
                    rewind.get_or_insert((before.clone(), out.clone()));
                }
            }
            // the same state again rumbles nothing
            assert!(watch.due(g).is_empty(), "tick {t}");
        }
        // a rollback: back to the tick before a thump, played again, rumbles nothing
        if let Some((g0, _)) = rewind {
            *app.world_mut().resource_mut::<Game>() = g0;
            watch.due(app.world().resource::<Game>());
            app.world_mut().run_schedule(FixedUpdate);
            app.world_mut().resource_mut::<Game>().match_stats.tick();
            assert!(watch.due(app.world().resource::<Game>()).is_empty(), "a rollback rumbled again");
        }
        serves.sort_by(|a, b| a.0.total_cmp(&b.0));
        eprintln!("{big} mistimed, {crisp} sweet, {quiet} quiet hits, {thumps} serve thumps; serve km/h {serves:?}");
        assert!(big + crisp + quiet > 10 && crisp > 0 && big > 0 && thumps > 0, "{big} {crisp} {quiet} {thumps}");
    }
}

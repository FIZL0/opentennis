//! The players' heads following the ball (`hst_sim::look`): after the motion step has posed each player, a player
//! whose tracking is on turns its head toward the ball (or, in doubles, back to the clip's pose while it is the
//! nearer of its pair to the net and the ball is behind it), at most 7° a tick. The turned head goes back into the
//! posed nodes, so the next tick's body hit tests the head where it is drawn, and to the drawn rig (`HeadTurn`).
//!
//! When it runs: one shared hold of 15 ticks after any contact in doubles (no player looks while it lasts); per
//! player a latch that a motion change clears and, mid-point, a blended change into a run, a stand or a stroke
//! that isn't a whiff turns on; a player not latched in a stroke latches on 6 ticks after its own contact, and
//! until then eases back to the clip's pose (except the server's first shot, a dive, a whiff or a smash, which
//! leave the head alone).

use bevy::prelude::*;
use hst_sim::look::{Look, Pose, Target};
use hst_sim::motion::id;
use hst_sim::world::{M4, mat_mul, rot_x, rot_y};

use super::{Figure, Game, Phase};
use crate::character::{HeadTurn, Motion};

/// One player's tracking: the look's state, the latch, the ticks since its own last contact, the motion set it
/// last saw and the nod and turn the head was drawn with this tick (None: the clip's pose).
#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
struct Seat {
    look: Look,
    latch: bool,
    since: u32,
    sets: u32,
    turned: Option<[f32; 2]>,
}

/// The shared contact hold, the shot count and phase a tick ago, and every player's tracking.
#[derive(Resource, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Looks {
    hold: i32,
    shots: i32,
    serve: bool,
    seats: Vec<Seat>,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Looks>().add_systems(FixedUpdate, (turn.after(super::bodyhit::pose).before(super::foot_fx::tick), show.after(turn)));
}

fn stroke(m: i32) -> bool {
    (0x10..=0x1f).contains(&m) || (0x25..=0x2a).contains(&m)
}

fn whiff(m: i32) -> bool {
    (0x27..=0x2a).contains(&m)
}

/// Each tick after the pose: the hold, the latches, and every looking player's head turned in `Posed`.
fn turn(mut looks: ResMut<Looks>, mut posed: ResMut<super::bodyhit::Posed>, bones: Res<super::bodyhit::Bones>, g: Res<Game>, q: Query<(&Figure, &Motion)>) {
    let n = g.players.len();
    let doubles = n > 2;
    let looks = &mut *looks;
    if looks.seats.len() != n {
        looks.seats = vec![Seat { since: u32::MAX, ..default() }; n];
        for s in &mut looks.seats {
            s.look.reset(false);
        }
    }
    let contact = g.shots != looks.shots;
    looks.shots = g.shots;
    // a new serve's set-up: the hold off, every player back to the start
    let serve = g.phase == Phase::Serve;
    let set_up = serve && !looks.serve;
    looks.serve = serve;
    looks.hold = (looks.hold - 1).max(0);
    if set_up {
        looks.hold = 0;
    } else if doubles && contact {
        looks.hold = hst_sim::look::HOLD;
    }
    for (f, m) in &q {
        let i = f.0;
        let Some(seat) = looks.seats.get_mut(i) else { continue };
        let p = &g.players[i];
        seat.since = if contact && g.last_hitter == i as i32 { 0 } else { seat.since.saturating_add(1) };
        let motion = m.id as i32;
        let dive = p.dive.is_some();
        let first = g.score.server == i as i32 && g.shots == 0;
        if set_up {
            seat.look.reset(false);
            seat.latch = false;
        } else if seat.sets != m.sets {
            seat.latch = false;
            seat.look.reseed = true;
            if (!first || !doubles) && !dive && motion != id::SMASH {
                seat.latch = m.fade.frames > 0 && (!stroke(motion) || !whiff(motion));
            } else {
                seat.look.reset(false);
            }
        }
        seat.sets = m.sets;
        seat.turned = None;
        if looks.hold > 0 {
            continue;
        }
        let target = if seat.latch {
            Some(aim(&g, i))
        } else if !stroke(motion) {
            None
        } else if !dive && !whiff(motion) && seat.since >= hst_sim::look::JUST_HIT as u32 {
            seat.latch = true;
            Some(aim(&g, i))
        } else if (first && seat.since < hst_sim::look::JUST_HIT as u32) || dive || whiff(motion) || motion == id::SMASH {
            None
        } else {
            Some(Target::Release)
        };
        let Some(target) = target else { continue };
        let Some(data) = g.data.get(i) else { continue };
        let sk = &data.skeleton;
        let at = bones.of(i, data).at;
        let (Some(head), Some(neck)) = (at[0], at[1]) else { continue };
        let Some(nodes) = posed.nodes_mut(i) else { continue };
        let clip_done = data.motions.get(&m.id).is_none_or(|c| c.length <= m.clock.sampled);
        let pose = Pose {
            clip: nodes.local[head],
            rest: sk.rest[head],
            parent: hst_sim::pose::node_world(sk, &nodes.local, neck, &super::player_matrix(p)),
            mirrored: p.hand < 0.0,
            clip_done,
            factor: 1.0,
        };
        if let Some(local) = seat.look.turn(&pose, target) {
            nodes.local[head] = local;
            seat.turned = Some(seat.look.applied);
        }
    }
}

/// Where a latched player looks: the ball, or in doubles mid-point back to the clip's pose while it is the nearer of
/// its pair to the net and the ball is on its side, deeper than it.
fn aim(g: &Game, i: usize) -> Target {
    let ball = g.flight.ball.pos;
    let ball = [ball[0], ball[1], ball[2], 1.0];
    let n = g.players.len();
    if n >= 3 && !matches!(g.phase, Phase::Post | Phase::ChangeEnds(_)) {
        let (z, partner) = (g.players[i].pos[2], g.players[(i + 2) % n].pos[2]);
        let side = |v: f32| v >= 0.0;
        if z.abs() < partner.abs() && side(ball[2]) == side(z) && z.abs() < ball[2].abs() {
            return Target::Release;
        }
    }
    Target::Ball(ball)
}

/// The drawn rig's head turn this tick, as a rotation in the head's own frame: the look's new local is that rotation
/// times the clip's (identity while the player doesn't look).
fn show(mut commands: Commands, looks: Res<Looks>, g: Res<Game>, q: Query<(Entity, &Figure, &Motion, Option<&HeadTurn>)>) {
    // drawn only in play: after the point and in a reaction, emote or celebration the clip keeps the head
    // (user 2026-10-27; the sim's head, which body hits read, still turns as the game's)
    let play = matches!(g.phase, Phase::Serve | Phase::Rally);
    for (e, f, m, shown) in &q {
        let Some(seat) = looks.seats.get(f.0) else { continue };
        let turned = seat.turned.filter(|_| play && g.players[f.0].emote == 0 && !(id::HIT_BY_BALL..=id::TEAM).contains(&(m.id as i32)));
        let r: M4 = turned.map_or(hst_sim::world::IDENTITY, |[nod, turn]| mat_mul(&rot_y(nod), &rot_x(turn)));
        let turn = HeadTurn(r);
        if shown != Some(&turn) {
            commands.entity(e).insert(turn);
        }
    }
}

impl super::snapshot::Wire for Looks {}

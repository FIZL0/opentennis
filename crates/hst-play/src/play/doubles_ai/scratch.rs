//! M185: the doubles stand test's leftover direction. When a computer player's NET/BASE stand searches all fail,
//! its base-line path test reads a scratch quad nothing in the routine set this tick; only the sign of its z counts.
//! The players are updated in court-slot order, and what each one finds there is what the player before it left:
//! the first player a crowd walker's spot (z never negative), the others the previous player's racket direction
//! (its `Racket` node's x axis in the world, times its hand), or on the tick that player tossed, the toss's launch
//! velocity, or on the tick its hit started the ball's trail, the trail colour (blue: never negative).

use super::*;
use crate::character::Motion;
use hst_sim::pose::{vmul, Nodes};

type M4 = [[f32; 4]; 4];

/// The arm joints the contact arm turn turns (`Motion::arm`'s order).
const ARM: [&str; 4] = ["Bip01RHand", "Bip01RForearm", "Bip01RUpperArm", "Bip01LUpperArm"];

/// What the stand test needs of each player's pose, and the toss and shot count at the end of the last tick (to
/// tell which player tossed or hit in this one).
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(in crate::play) struct Left {
    /// Per player its pose as its last motion step left it, kept to the `Racket` chain.
    #[serde(default)]
    posed: Vec<Option<Kept>>,
    tossed: bool,
    shots: i32,
}

/// A player's `Racket` node and its ancestors up to the root (node, local rows, held rotation and position) as its
/// motion step posed them, before the arm turn, and its motion after that step.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Kept {
    chain: Vec<(usize, M4, [f32; 4], [f32; 3])>,
    motion: Motion,
}

/// Player `i`'s racket direction z: its `Racket` node's world x axis (the chain under its placement, with the
/// contact or dive arm turn and a dive's racket turn it holds now), z, times its hand.
fn racket(g: &Game, i: usize) -> Option<f32> {
    let chain = this_tick(g, i)?;
    let (p, sk) = (&g.players[i], &g.data.get(i)?.skeleton);
    let turned = |n: usize, local: &M4| -> M4 {
        let Some((quats, w)) = p.arm else { return *local };
        let Some(j) = ARM.iter().position(|a| sk.names[n] == *a) else { return *local };
        let mut m = vmul(local, &hst_sim::quat::to_matrix(hst_sim::quat::slerp([0.0, 0.0, 0.0, 1.0], quats[j], w)));
        m[3] = local[3];
        m
    };
    // a contact dive turns the racket's own local first (B716), keeping its translation
    let racket = p.racket.map_or(chain[0].1, |r| [r[0], r[1], r[2], chain[0].1[3]]);
    let mut m = turned(chain[0].0, &racket);
    for (n, local) in &chain[1..] {
        m = vmul(&m, &turned(*n, local));
    }
    let world = vmul(&m, &super::super::player_matrix(p));
    Some(hst_sim::ps2::mul(p.hand, world[0][2]))
}

/// The z the stand test of computer player `i` finds this tick (only its sign is read).
pub(super) fn z(g: &Game, i: usize) -> f32 {
    let Some(before) = i.checked_sub(1) else { return 1.0 };
    let left = &g.doubles_ai.left;
    if g.serving.tossed && !left.tossed && g.score.server == before as i32 {
        return g.flight.ball.vel[2];
    }
    if g.shots != left.shots && g.last_hitter == before as i32 {
        return 1.0;
    }
    racket(g, before).unwrap_or(1.0)
}

/// After the pose: the toss and shot count the tick ends with, and each player's racket chain and motion as its
/// motion step left them.
pub(in crate::play) fn after_pose(mut g: ResMut<Game>, posed: Res<super::super::bodyhit::Posed>, q: Query<(&Figure, &Motion)>) {
    let g = &mut *g;
    (g.doubles_ai.left.tossed, g.doubles_ai.left.shots) = (g.serving.tossed, g.shots);
    if g.players.len() != 4 {
        return;
    }
    g.doubles_ai.left.posed.resize(4, None);
    for (f, m) in &q {
        let i = f.0;
        let kept = g.data.get(i).zip(posed.nodes(i)).and_then(|(data, nodes)| {
            let chain = racket_chain(&data.skeleton)?.into_iter().map(|k| (k, nodes.local[k], nodes.held(k).0, nodes.held(k).1)).collect();
            Some(Kept { chain, motion: *m })
        });
        g.doubles_ai.left.posed[i] = kept;
    }
}

/// The `Racket` node and its ancestors up to the root.
fn racket_chain(sk: &hst_sim::pose::Skeleton) -> Option<Vec<usize>> {
    let mut n = sk.names.iter().position(|b| b == "Racket")?;
    let mut chain = vec![n];
    while let Some(p) = sk.parent[n] {
        chain.push(p);
        n = p;
    }
    Some(chain)
}

/// Player `i`'s `Racket` chain as its own update poses it in this tick (B773): every player's update steps its motion
/// before the next player's AI runs, so the pose is the kept one with player `i`'s motion command for this tick
/// played (a motion it started this tick from its first frame, with its crossfade or cut) and one motion step run,
/// its clock, crossfade and held fade as `character::tick` runs them.
fn this_tick(g: &Game, i: usize) -> Option<Vec<(usize, M4)>> {
    let kept = g.doubles_ai.left.posed.get(i)?.as_ref()?;
    let data = g.data.get(i)?;
    let mut m = kept.motion;
    crate::play::motion_command(g, i, &mut m);
    let mut nodes = Nodes::new(&data.skeleton);
    for &(n, local, q, p) in &kept.chain {
        nodes.restore(n, local, (q, p));
    }
    let keep = |k: usize| kept.chain.iter().any(|c| c.0 == k);
    let new = data.motions.get(&m.id)?;
    // a motion started this tick binds its clip first (B777): rest rows, single-key tracks at their key
    if m.id != kept.motion.id {
        nodes.bind(&data.skeleton, new);
    }
    let (mut clock, mut fade) = (m.clock, m.fade);
    let old = data.motions.get(&(fade.id as usize)).map_or(0.0, |c| c.length);
    let mut mix = fade.tick(old).map(|w| (w, false));
    clock.tick(new.length);
    if let Some(w) = fade.tick_hold() {
        mix = Some((w, true));
    }
    let out = data.motions.get(&(fade.mixed as usize));
    match (mix, data.motions.get(&(fade.id as usize)).filter(|_| fade.clip), out) {
        (Some((w, true)), Some(old), _) => {
            nodes.sample_kept(old, fade.sampled, keep);
            nodes.mix_kept(new, 0.0, w, 23, keep);
        }
        (Some((w, false)), _, Some(old)) => {
            nodes.sample_kept(new, clock.sampled, keep);
            nodes.mix_kept(old, fade.sampled, w, 23, keep);
        }
        _ => nodes.sample_kept(new, clock.sampled, keep),
    }
    Some(kept.chain.iter().map(|c| (c.0, nodes.local[c.0])).collect())
}

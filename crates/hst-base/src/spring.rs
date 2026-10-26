//! Spring bones (character standard §3b), mods only: a joint with `extras.spring` is a point mass hung on its
//! parent joint. Each drawn frame, after the pose, the mass trails where the pose puts it (pulled back by
//! `stiffness`, velocity kept by `damping`, sagging by `gravity`, at most `limit` degrees off the posed direction)
//! and its parent turns to point at it; a chain's first record (whose parent has none) is its pinned anchor. Out of Bounds' chains carry these (its `.rem` dynamics records); the
//! game's own solver is not ported (notes/oob.md *Dynamics*), so this is a plain mass-spring that looks the part.
//! Masses are pushed out of the source's collision ellipsoids (§3c, `extras.colliders`) on their joints: what keeps
//! Out of Bounds' hair out of a hat or the head.
//! Stock characters have no spring joints: their pose and the sim are untouched.

use bevy::prelude::*;

use crate::character::{Joint, Rig};

/// A spring joint: the joint, its parent (the one that turns) and its §3b parameters.
pub struct Mass {
    pub joint: usize,
    pub parent: usize,
    limit: f32,
    stiffness: f32,
    gravity: f32,
    damping: f32,
    /// The chain's first link (hung on the pinned anchor).
    first: bool,
}

/// The skin's spring joints (`nodes` in skin order, as `joints`), parents before children.
pub fn masses(nodes: &[gltf::Node], joints: &[Joint]) -> Vec<Mass> {
    let depth = |mut j: usize| {
        let mut d = 0;
        while let Some(p) = joints[j].parent {
            j = p;
            d += 1;
        }
        d
    };
    let mut out: Vec<(usize, Mass)> = nodes
        .iter()
        .enumerate()
        .filter_map(|(k, n)| {
            let e: serde_json::Value = serde_json::from_str(n.extras().as_ref()?.get()).ok()?;
            let s = &e["spring"];
            let f = |key: &str, default: f64| s[key].as_f64().unwrap_or(default) as f32;
            s.is_object().then_some(())?;
            Some((depth(k), Mass { joint: k, parent: joints[k].parent?, limit: f("limit", 30.0).to_radians(), stiffness: f("stiffness", 0.5), gravity: f("gravity", 4.9), damping: f("damping", 0.83), first: false }))
        })
        .collect();
    // a chain's first record is its anchor (stiffness 1, limit 0..5°): pinned, it never turns the head it hangs on
    let springy: Vec<usize> = out.iter().map(|(_, m)| m.joint).collect();
    out.retain(|(_, m)| springy.contains(&m.parent));
    out.sort_by_key(|(d, _)| *d);
    let inner: Vec<usize> = out.iter().map(|(_, m)| m.joint).collect();
    out.into_iter().map(|(_, m)| Mass { first: !inner.contains(&m.parent), ..m }).collect()
}

/// A mass in world space: position, last step's motion, and its parent joint's world matrix at that step (the frame
/// it is carried in).
type State = (Vec3, Vec3, Mat4);

/// A rig's masses; `None` until the first pose.
#[derive(Component, Default)]
pub struct Spring {
    state: Vec<Option<State>>,
    clock: f32,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (attach, swing.after(crate::character::animate).before(crate::noise::draw)));
}

fn springs(rig: &Rig) -> (&[Mass], &[(usize, Mat4)]) {
    rig.data.noise.as_ref().map_or((&[], &[]), |c| (&c.springs, &c.colliders))
}

fn attach(mut commands: Commands, rigs: Query<(Entity, &Rig), Added<Rig>>) {
    // HST_NO_SPRINGS=1: the rigid bind-pose hair, to compare against
    if std::env::var_os("HST_NO_SPRINGS").is_some() {
        return;
    }
    for (root, rig) in &rigs {
        if !springs(rig).0.is_empty() {
            commands.entity(root).try_insert(Spring::default());
        }
    }
}

// ponytail: Out of Bounds' stiffness read per 60 Hz step rang at ~7 Hz (B410b); a tenth of it and the lag cap were picked
// by eye on Sophie idling. The game's own solver would replace both
const SOFT: f32 = 0.1;
const LAG: f32 = 3f32.to_radians();
/// M132: a lower link's lag grows by at most this a step (picked with the user against Out of Bounds' footage): the
/// pigtails fling out on a swing as there, and still hang still on an idle bob
const CHILD_LAG: f32 = 2f32.to_radians();
/// Steps a fresh rig's hair hangs for at its first pose: soft springs take ~2 s to fall from the bind pose, and a
/// preview bake or a point's set-up would catch it sticking out
const HANG: usize = 180;

/// One 60 Hz step of a mass toward `target`, hung at `origin`; returns the new position and motion.
/// The skin's collision ellipsoids (standard §3c): (joint, matrix in its space mapping the unit sphere onto it).
pub fn colliders(nodes: &[gltf::Node]) -> Vec<(usize, Mat4)> {
    nodes
        .iter()
        .enumerate()
        .filter_map(|(k, n)| Some((k, serde_json::from_str::<serde_json::Value>(n.extras().as_ref()?.get()).ok()?)))
        .flat_map(|(k, e)| {
            let list = e["colliders"].as_array().cloned().unwrap_or_default();
            list.into_iter().filter_map(move |c| {
                let v: Vec<f32> = c.as_array()?.iter().filter_map(|x| x.as_f64().map(|x| x as f32)).collect();
                Some((k, Mat4::from_cols_slice(v.get(..16)?)))
            })
        })
        .collect()
}

/// `x` (at its segment's length from `origin`) pushed out of each ellipsoid (world matrix, its inverse) that holds
/// it, unless the pose puts `target` inside it too (a skirt's thigh ellipsoid around a hanging tip: the collider
/// doesn't fight the pose).
fn outside(mut x: Vec3, origin: Vec3, target: Vec3, colliders: &[(Mat4, Mat4)]) -> Vec3 {
    let len = (target - origin).length();
    // ponytail: out to the surface, back to the segment's length, a few rounds: a tip that can't get out (an
    // anchor inside the ball) stays near it; a proper sphere-segment solve if hair still pokes in
    for _ in 0..4 {
        let mut pushed = false;
        for (c, inv) in colliders {
            let u = inv.transform_point3(x);
            if u.length_squared() < 1.0 && inv.transform_point3(target).length_squared() >= 1.0 {
                x = c.transform_point3(u.normalize_or(Vec3::Y) * 1.001);
                pushed = true;
            }
        }
        if !pushed {
            break;
        }
        x = origin + (x - origin).normalize_or_zero() * len;
    }
    x
}

fn step(m: &Mass, origin: Vec3, target: Vec3, (x, v): (Vec3, Vec3)) -> (Vec3, Vec3) {
    let len = (target - origin).length();
    let v = v * m.damping + (target - x) * m.stiffness * SOFT + Vec3::NEG_Y * m.gravity / 3600.0;
    let posed = (target - origin).normalize_or_zero();
    // only the motion across the segment turns it: a pull along it (gravity on a tip posed upright) would shorten
    // the step and the renormalised turn would overshoot, swinging for ever
    let r = (x - origin).normalize_or(posed);
    // ponytail: at most half the segment a step, so a whipped short one (an earring's 8 mm tip) can't flip through
    // its anchor; a sub-stepped solver if fast swings ever look stiff
    let v = (v - r * v.dot(r)).clamp_length_max(len * 0.5);
    let mut dir = (x + v - origin).normalize_or(posed);
    let off = posed.angle_between(dir);
    if off > m.limit {
        dir = Quat::from_rotation_arc(posed, dir).slerp(Quat::IDENTITY, 1.0 - m.limit / off) * posed;
    }
    let y = origin + dir * len;
    (y, y - x)
}

/// The local rotation of a joint under `above` (its parent's world matrix) whose world rotation is turned by `turn`.
fn turned(above: Mat4, turn: Quat, local: Quat) -> Quat {
    let a = Mat3::from_mat4(above);
    Quat::from_mat3(&(a.inverse() * Mat3::from_quat(turn) * a * Mat3::from_quat(local))).normalize()
}

pub fn swing(time: Res<Time>, mut rigs: Query<(&Rig, &GlobalTransform, &mut Spring, Has<crate::shadow::Minor>, Option<&Visibility>)>, mut joints: Query<&mut Transform>, mut locals: Local<Vec<Transform>>) {
    for (rig, global, mut s, minor, shown) in &mut rigs {
        // not posed while put away (`character::animate`), so not swung on top of the last pose either
        if crate::character::put_away(minor, shown) {
            continue;
        }
        let (masses, colliders) = springs(rig);
        // ponytail: whole 60 Hz steps against this frame's pose (none while paused); sub-frame targets if it judders
        s.clock += time.delta_secs() * 60.0;
        let steps = (s.clock.floor() as usize).min(4);
        s.clock = (s.clock - s.clock.floor()).min(1.0);
        locals.clear();
        locals.extend(rig.joints.iter().map(|&e| joints.get(e).copied().unwrap_or_default()));
        let parents: Vec<Option<usize>> = rig.data.joints.iter().map(|j| j.parent).collect();
        solve(masses, colliders, &parents, global.to_matrix(), &mut locals, &mut s.state, steps);
        for m in masses {
            if let Ok(mut tf) = joints.get_mut(rig.joints[m.parent]) {
                tf.set_if_neq(locals[m.parent]);
            }
        }
    }
}

/// One drawn frame of `masses` on a skeleton (`parents`, posed `locals`) under `root`: `steps` 60 Hz steps, then each
/// spring's parent joint in `locals` turned to point at its mass.
fn solve(masses: &[Mass], colliders: &[(usize, Mat4)], parents: &[Option<usize>], root: Mat4, locals: &mut [Transform], state: &mut Vec<Option<State>>, steps: usize) {
    state.resize(masses.len(), None);
    let mut world: Vec<Option<Mat4>> = vec![None; locals.len()];
    fn posed(j: usize, parents: &[Option<usize>], root: Mat4, locals: &[Transform], w: &mut [Option<Mat4>]) -> Mat4 {
        if let Some(m) = w[j] {
            return m;
        }
        let m = parents[j].map_or(root, |p| posed(p, parents, root, locals, w)) * locals[j].to_matrix();
        w[j] = Some(m);
        m
    }
    // on core joints (head, spine), which the springs never turn: posed once for the frame
    let colliders: Vec<(Mat4, Mat4)> = colliders
        .iter()
        .map(|&(j, c)| {
            let w = posed(j, parents, root, locals, &mut world) * c;
            (w, w.inverse())
        })
        .collect();
    for (m, st) in masses.iter().zip(state.iter_mut()) {
        let frame = posed(m.parent, parents, root, locals, &mut world);
        let target = posed(m.joint, parents, root, locals, &mut world).w_axis.truncate();
        let origin = frame.w_axis.truncate();
        // a fresh rig, or one placed somewhere new (a point's set-up), starts at the pose
        let kept = st.filter(|(x, ..)| x.distance(target) < 1.0);
        let steps = if kept.is_some() { steps } else { HANG };
        let (x, v, last) = kept.unwrap_or((target, Vec3::ZERO, frame));
        // B410b: carried with its parent joint since the last step, keeping some of its world motion (the head moves
        // on under it: the swing). Fully kept, every link would lag its own origin again and a short chain whip off a
        // gentle bob; fully carried (B410b's lower links), the pigtails hang stiff through a swing (M132)
        let moved = frame * last.inverse();
        let (cx, cv) = (moved.transform_point3(x), moved.transform_vector3(v));
        let mut dir = (cx - origin).normalize_or_zero();
        if steps > 0 {
            // the lag grows by at most `LAG` (`CHILD_LAG` below the first link) a step: a jolt of the head would flick a
            // short ribbon
            let lag = (x - origin).angle_between(cx - origin);
            let cap = if m.first { LAG } else { CHILD_LAG };
            let c = if lag > cap { 1.0 - cap / lag } else { 0.0 };
            let (mut x, mut v) = (x.lerp(cx, c), v.lerp(cv, c));
            for _ in 0..steps {
                (x, v) = step(m, origin, target, (x, v));
                x = outside(x, origin, target, &colliders);
            }
            *st = Some((x, v, frame));
            dir = (x - origin).normalize_or_zero();
        }
        // between steps the hair keeps the last step's direction in its parent's frame, within this pose's limit
        let posed_dir = (target - origin).normalize_or_zero();
        if dir == Vec3::ZERO {
            continue;
        }
        let off = posed_dir.angle_between(dir);
        if off > m.limit {
            dir = Quat::from_rotation_arc(posed_dir, dir).slerp(Quat::IDENTITY, 1.0 - m.limit / off) * posed_dir;
        }
        let turn = Quat::from_rotation_arc(posed_dir, dir);
        if turn.is_nan() || turn.angle_between(Quat::IDENTITY) < 1e-6 {
            continue;
        }
        // the parent's world rotation turned by `turn`, back in its parent's frame: through the whole matrix, as a
        // left-hander's root mirrors (scale x −1) and a decomposed rotation would turn the hair the wrong way
        let above = parents[m.parent].map_or(root, |p| posed(p, parents, root, locals, &mut world));
        locals[m.parent].rotation = turned(above, turn, locals[m.parent].rotation);
        world[m.parent] = Some(above * locals[m.parent].to_matrix());
        world[m.joint] = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mass(limit: f32) -> Mass {
        Mass { joint: 1, parent: 0, limit: limit.to_radians(), stiffness: 0.5, gravity: 4.9, damping: 0.83, first: true }
    }

    #[test]
    fn trails_then_settles_within_the_limit() {
        let m = mass(30.0);
        let origin = Vec3::ZERO;
        // the head jerks sideways: the posed tip jumps, the mass lags, bounded by the limit, then settles back
        let mut st = (Vec3::NEG_Y * 0.2, Vec3::ZERO);
        let target = Vec3::new(0.2, 0.0, 0.0);
        st = step(&m, origin, target, st);
        let off = target.angle_between(st.0);
        assert!(off > 0.1 && off <= 30f32.to_radians() + 1e-4, "{off}");
        assert!((st.0.length() - 0.2).abs() < 1e-5);
        for _ in 0..600 {
            st = step(&m, origin, target, st);
        }
        // at rest, sagging off the target by gravity over the (softened) stiffness
        let rest = step(&m, origin, target, st);
        assert!(rest.0.distance(st.0) < 1e-5 && target.angle_between(st.0) < 30f32.to_radians(), "{:?}", st.0);
    }

    #[test]
    fn a_turn_lands_under_a_mirrored_parent() {
        // B410: a left-hander's root has scale x −1; the turned joint's child must end up where the world turn puts it
        let local = Quat::from_rotation_z(0.4) * Quat::from_rotation_x(-0.2);
        let child = Vec3::new(0.0, -0.1, 0.02);
        let turn = Quat::from_rotation_arc(Vec3::NEG_Y, Vec3::new(0.3, -1.0, 0.2).normalize());
        for hand in [1.0, -1.0] {
            let above = Mat4::from_scale_rotation_translation(Vec3::new(hand, 1.0, 1.0), Quat::from_rotation_y(2.0) * Quat::from_rotation_x(0.3), Vec3::new(3.0, 1.0, 12.0));
            let tip = |q: Quat| above.transform_vector3(q * child);
            let want = turn * tip(local);
            let got = tip(turned(above, turn, local));
            assert!(got.distance(want) < 1e-5, "hand {hand}: {got} vs {want}");
        }
    }

    /// A chain hung off a head at `head(frame)` (translation), one step a frame through `solve`; returns each frame's
    /// largest angle of a link off its pose.
    fn chain(params: &[(f32, f32, f32)], seg: f32, frames: usize, head: impl Fn(usize) -> Vec3) -> Vec<f32> {
        let masses: Vec<Mass> = params.iter().enumerate().map(|(k, &(limit, stiffness, damping))| Mass { joint: k + 1, parent: k, limit: limit.to_radians(), stiffness, gravity: 4.9, damping, first: k == 0 }).collect();
        let parents: Vec<Option<usize>> = (0..=params.len()).map(|k| k.checked_sub(1)).collect();
        // rest: hanging out sideways and down
        let rest: Vec<Transform> = (0..=params.len()).map(|k| Transform::from_xyz(seg * 0.7 * (k > 0) as u8 as f32, -seg * 0.7 * (k > 0) as u8 as f32, 0.0)).collect();
        let mut state = Vec::new();
        (0..frames)
            .map(|f| {
                let root = Mat4::from_translation(head(f));
                let mut locals = rest.clone();
                solve(&masses, &[], &parents, root, &mut locals, &mut state, 1);
                let world = |l: &[Transform]| l.iter().scan(root, |w, t| { *w *= t.to_matrix(); Some(w.w_axis.truncate()) }).collect::<Vec<_>>();
                let (posed, now) = (world(&rest), world(&locals));
                (1..now.len()).map(|j| (now[j] - now[j - 1]).angle_between(posed[j] - posed[j - 1])).fold(0f32, f32::max)
            })
            .collect()
    }

    /// Sophie's pigtail (L_kami01..10): limit, stiffness, damping.
    const KAMI: [(f32, f32, f32); 10] = [(5.0, 0.7, 0.83), (30.0, 0.5, 0.83), (20.0, 0.4, 0.73), (20.0, 0.3, 0.8), (20.0, 0.3, 0.83), (20.0, 0.5, 0.83), (20.0, 0.2, 0.83), (5.0, 0.7, 0.83), (0.0, 0.7, 0.83), (0.0, 0.7, 0.83)];

    #[test]
    fn a_still_chain_settles() {
        // the head jumps 30 cm across, then holds still: the tail swings out, then comes to rest (sagging, each link from the one above)
        let off = chain(&KAMI, 0.04, 600, |f| if f < 10 { Vec3::ZERO } else { Vec3::X * 0.3 });
        assert!(off[11..30].iter().any(|&o| o > 20f32.to_radians()), "{:?}", &off[11..30]);
        assert!((off[599] - off[400]).abs() < 1e-3 && off[599] < 60f32.to_radians(), "{} {}", off[400].to_degrees(), off[599].to_degrees());
    }

    #[test]
    fn a_jump_bends_the_lower_links() {
        // M132: the head jumps 30 cm across: a link below the first trails its own origin too (the pigtail flings out),
        // within its limit, and comes back to its pose
        let masses: Vec<Mass> = KAMI.iter().enumerate().map(|(k, &(limit, stiffness, damping))| Mass { joint: k + 1, parent: k, limit: limit.to_radians(), stiffness, gravity: 4.9, damping, first: k == 0 }).collect();
        let parents: Vec<Option<usize>> = (0..=KAMI.len()).map(|k| k.checked_sub(1)).collect();
        let rest: Vec<Transform> = (0..=KAMI.len()).map(|k| Transform::from_xyz(0.0, -0.04 * (k > 0) as u8 as f32, 0.0)).collect();
        let mut state = Vec::new();
        // link 3's local turn off its pose, each frame
        let bend: Vec<f32> = (0..300)
            .map(|f| {
                let mut locals = rest.clone();
                solve(&masses, &[], &parents, Mat4::from_translation(if f < 10 { Vec3::ZERO } else { Vec3::X * 0.3 }), &mut locals, &mut state, 1);
                locals[3].rotation.angle_between(Quat::IDENTITY)
            })
            .collect();
        let most = bend.iter().cloned().fold(0f32, f32::max);
        assert!(most > 5f32.to_radians() && most <= 20.5f32.to_radians(), "{}", most.to_degrees());
        assert!(bend[299] < 3f32.to_radians(), "{}", bend[299].to_degrees());
    }

    #[test]
    fn a_collider_keeps_the_mass_out() {
        // a link posed out sideways sags under gravity into a 9 cm ball below it (on the root joint): with the
        // collider it settles on the ball's surface, not in it
        let m = Mass { joint: 1, parent: 0, limit: 60f32.to_radians(), stiffness: 0.5, gravity: 20.0, damping: 0.83, first: true };
        let parents = [None, Some(0)];
        let rest = [Transform::IDENTITY, Transform::from_xyz(0.1, 0.0, 0.0)];
        let ball = Mat4::from_scale_rotation_translation(Vec3::splat(0.09), Quat::IDENTITY, Vec3::new(0.1, -0.1, 0.0));
        let centre = ball.w_axis.truncate();
        let settle = |colliders: &[(usize, Mat4)]| {
            let mut state = Vec::new();
            let mut locals = rest;
            for _ in 0..600 {
                locals = rest;
                solve(std::slice::from_ref(&m), colliders, &parents, Mat4::IDENTITY, &mut locals, &mut state, 1);
            }
            (locals[0].to_matrix() * locals[1].to_matrix()).w_axis.truncate().distance(centre)
        };
        assert!(settle(&[]) < 0.085, "{}", settle(&[]));
        assert!(settle(&[(0, ball)]) > 0.0895, "{}", settle(&[(0, ball)]));
    }

    #[test]
    fn zero_limit_is_rigid() {
        let m = mass(0.0);
        let st = step(&m, Vec3::ZERO, Vec3::X, (Vec3::NEG_Y, Vec3::ZERO));
        assert!(st.0.distance(Vec3::X) < 1e-5);
    }

    #[test]
    fn a_bobbing_head_does_not_whip() {
        // B410b: an idle bob (3 cm at ~3 Hz) lagged every 4 cm link off its own origin: the pigtail whipped up to 86°
        // off its pose, 38° in one step. Carried by the link above (lower links lag ≤ `CHILD_LAG` a step), it sways:
        // ≤ 5.9° a step with M132's lower-link lag
        let off = chain(&KAMI, 0.04, 600, |f| Vec3::Y * 0.03 * (f as f32 * 0.3).sin());
        let jump = off[300..].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
        assert!(jump < 8f32.to_radians() && off[300..].iter().all(|&o| o < 60f32.to_radians()), "{} {:?}", jump.to_degrees(), off[300..].iter().cloned().fold(0f32, f32::max).to_degrees());
    }
}

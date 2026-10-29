//! Spring bones (character standard §3b), mods only: Out of Bounds' hair chains, solved the way that game solves them
//! (notes/oob.md *Chain solver*). A chain is a path of `extras.spring` joints from its anchor (a spring joint whose
//! parent has none) down, plus a free tip one last-bone offset past its last joint. Each joint but the anchor is a
//! point mass under gravity, held to its neighbours by length springs and dragged along by its frame's motion; after
//! every 60 Hz step each bone turns toward the next mass, at most its limit off its bind direction in its solved
//! parent's frame, and the masses go back where the turned bones put them. No pull toward the pose: the rigid bones
//! and the cones keep the shape. A mass that ends a step inside one of the character's ellipsoids (§3c) is pushed onto
//! its surface, so hair stays out of the head and a hat. Stock characters have no spring joints: their pose and the sim are untouched.

use bevy::prelude::*;

use crate::character::{Joint, Rig};

/// Out of Bounds' own per-joint words (notes/oob.md *Param words*), for a §3b joint without them.
const DEFAULT: [f32; 16] = [40.0, 0.65, 9.8, 0.5, 0.0, 500.0, 1.0, 0.83, 0.2, 0.0, 0.2, 0.8, 0.0, 0.0, 0.0, 0.0];
const LIMIT: usize = 0;
const CARRY: usize = 1;
const GRAVITY: usize = 2;
const MASS: usize = 3;
const STIFF: usize = 5;
const DAMP: usize = 6;
const KEEP: usize = 7;
const FEEDBACK: usize = 8;
const BOUNCE: usize = 10;
const SLIDE: usize = 11;
const DT: f32 = 1.0 / 60.0;
/// Settle calls (two steps each) a fresh chain runs from its bind pose, as the game's reset does.
const SETTLE: usize = 10;

/// A chain: its joints (skin order), anchor first, and each one's words; the tip reuses the last joint's.
pub struct Chain {
    pub joints: Vec<usize>,
    words: Vec<[f32; 16]>,
}

/// The skin's spring chains (`nodes` in skin order, as `joints`), anchors nearest the root first.
pub fn chains(nodes: &[gltf::Node], joints: &[Joint]) -> Vec<Chain> {
    let words: Vec<Option<[f32; 16]>> = nodes
        .iter()
        .map(|n| {
            let e: serde_json::Value = serde_json::from_str(n.extras().as_ref()?.get()).ok()?;
            let s = &e["spring"];
            s.is_object().then_some(())?;
            if let Some(w) = s["oob"].as_array().filter(|w| w.len() == 16) {
                return Some(std::array::from_fn(|k| w[k].as_f64().unwrap_or(0.0) as f32));
            }
            // §3b's keys over the defaults
            let mut w = DEFAULT;
            for (key, k) in [("limit", LIMIT), ("stiffness", CARRY), ("gravity", GRAVITY), ("damping", KEEP)] {
                w[k] = s[key].as_f64().map_or(w[k], |v| v as f32);
            }
            Some(w)
        })
        .collect();
    let sprung = |j: Option<usize>| j.is_some_and(|j| words[j].is_some());
    let depth = |mut j: usize| std::iter::from_fn(|| joints[j].parent.inspect(|&p| j = p)).count();
    let mut out = Vec::new();
    let mut anchors: Vec<usize> = (0..joints.len()).filter(|&j| sprung(Some(j)) && !sprung(joints[j].parent)).collect();
    anchors.sort_by_key(|&j| depth(j));
    // ponytail: a branch (two spring children) makes one chain per path, the shared joints solved by each in turn
    fn walk(j: usize, path: &mut Vec<usize>, joints: &[Joint], sprung: &dyn Fn(Option<usize>) -> bool, out: &mut Vec<Vec<usize>>) {
        path.push(j);
        let kids: Vec<usize> = (0..joints.len()).filter(|&k| joints[k].parent == Some(j) && sprung(Some(k))).collect();
        if kids.is_empty() {
            out.push(path.clone());
        }
        for k in kids {
            walk(k, path, joints, sprung, out);
        }
        path.pop();
    }
    for a in anchors {
        walk(a, &mut Vec::new(), joints, &sprung, &mut out);
    }
    out.into_iter().map(|joints| Chain { words: joints.iter().map(|&j| words[j].unwrap()).collect(), joints }).collect()
}

/// §3c's ellipsoids: (joint, matrix in its space that maps the unit sphere onto it), from every node's
/// `extras.colliders`.
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

/// A chain's state, joints `0..=n` (the last is the tip): world positions and velocities, the bones' solved local
/// rotations (`0..n`), rest lengths to the joint above, and the root frame at the last call.
#[derive(Clone)]
struct Run {
    p: Vec<Vec3>,
    v: Vec<Vec3>,
    rot: Vec<Quat>,
    rest: Vec<f32>,
    last: Mat4,
}

/// A rig's chains; `None` until the first pose.
#[derive(Component, Default)]
pub struct Spring {
    runs: Vec<Option<Run>>,
    /// The colliders' world matrices at the last call.
    shapes: Vec<Mat4>,
    clock: f32,
    /// The motion set last seen, and whether the next call follows a new one.
    sets: u32,
    cut: bool,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (attach, swing.after(crate::character::animate).before(crate::noise::draw)));
}

fn springs(rig: &Rig) -> &[Chain] {
    rig.data.noise.as_ref().map_or(&[], |c| &c.springs)
}

fn ellipsoids(rig: &Rig) -> &[(usize, Mat4)] {
    rig.data.noise.as_ref().map_or(&[], |c| &c.colliders)
}

fn attach(mut commands: Commands, rigs: Query<(Entity, &Rig), Added<Rig>>) {
    // HST_NO_SPRINGS=1: the rigid bind-pose hair, to compare against
    if std::env::var_os("HST_NO_SPRINGS").is_some() {
        return;
    }
    for (root, rig) in &rigs {
        if !springs(rig).is_empty() {
            commands.entity(root).try_insert(Spring::default());
        }
    }
}

pub fn swing(time: Res<Time>, mut rigs: Query<(&Rig, &GlobalTransform, &mut Spring, Has<crate::shadow::Minor>, Option<&Visibility>, Option<&crate::character::Motion>)>, mut joints: Query<&mut Transform>, mut locals: Local<Vec<Transform>>) {
    for (rig, global, mut s, minor, shown, motion) in &mut rigs {
        // not posed while put away (`character::animate`), so not swung on top of the last pose either
        if crate::character::put_away(minor, shown) {
            continue;
        }
        // the game cuts between most motions: the head jumps in a frame, and dragged by only a share of that the hair
        // flew up as one began or ended. The step after a new one carries the hair whole, as a placement would.
        if let Some(m) = motion
            && m.sets != s.sets
        {
            (s.sets, s.cut) = (m.sets, true);
        }
        s.clock += time.delta_secs() * 60.0;
        let steps = (s.clock.floor() as usize).min(4);
        s.clock = (s.clock - s.clock.floor()).min(1.0);
        locals.clear();
        locals.extend(rig.joints.iter().map(|&e| joints.get(e).copied().unwrap_or_default()));
        let rest: Vec<Transform> = rig.data.joints.iter().map(|j| j.rest).collect();
        let parents: Vec<Option<usize>> = rig.data.joints.iter().map(|j| j.parent).collect();
        let s = &mut *s;
        solve(springs(rig), ellipsoids(rig), &parents, &rest, global.to_matrix(), &mut locals, (&mut s.runs, &mut s.shapes), steps, s.cut);
        s.cut &= steps == 0;
        for c in springs(rig) {
            for &j in &c.joints {
                if let Ok(mut tf) = joints.get_mut(rig.joints[j]) {
                    tf.set_if_neq(locals[j]);
                }
            }
        }
    }
}

/// One drawn frame of `chains` and `colliders` on a skeleton (`parents`, bind `rest`, posed `locals`) under `root`:
/// `steps` 60 Hz steps (none: the bones keep their last turn), then each chain joint's local in `locals` set to its
/// solved bone. `shapes` holds the colliders' world matrices from the last call; after a `cut` the chains move
/// whole with their frames.
#[allow(clippy::too_many_arguments)]
fn solve(chains: &[Chain], colliders: &[(usize, Mat4)], parents: &[Option<usize>], rest: &[Transform], root: Mat4, locals: &mut [Transform], (runs, shapes): (&mut Vec<Option<Run>>, &mut Vec<Mat4>), steps: usize, cut: bool) {
    runs.resize(chains.len(), None);
    let world = |mut j: Option<usize>, locals: &[Transform]| {
        let mut m = Mat4::IDENTITY;
        while let Some(k) = j {
            m = locals[k].to_matrix() * m;
            j = parents[k];
        }
        root * m
    };
    let now: Vec<Mat4> = colliders.iter().map(|&(j, m)| world(Some(j), locals) * m).collect();
    if shapes.len() != now.len() {
        shapes.clone_from(&now);
    }
    let sweep = (&shapes[..], &now[..]);
    for (c, run) in chains.iter().zip(runs.iter_mut()) {
        let n = c.joints.len();
        // joint j's bind local; the tip's is the last bone's
        let bind = |j: usize| rest[c.joints[j.min(n - 1)]];
        let r = world(parents[c.joints[0]], locals);
        // a fresh rig, or one placed somewhere new (a point's set-up), starts from the bind pose and settles
        let run = match run.take().filter(|u| u.last.w_axis.distance(r.w_axis) < 1.0) {
            Some(u) => run.insert(u),
            None => {
                let u = run.insert(reset(c, &bind, r));
                for _ in 0..SETTLE {
                    call(c, &bind, u, r, (&now, &now), 2, false);
                }
                u
            }
        };
        if steps > 0 {
            call(c, &bind, run, r, sweep, steps, cut);
        }
        for (j, &k) in c.joints.iter().enumerate() {
            locals[k] = Transform { rotation: run.rot[j], ..bind(j) };
        }
    }
    *shapes = now;
}

/// A chain on its bind pose under root frame `r`, at rest.
fn reset(c: &Chain, bind: &dyn Fn(usize) -> Transform, r: Mat4) -> Run {
    let n = c.joints.len();
    let mut above = r;
    let p: Vec<Vec3> = (0..=n)
        .map(|j| {
            above *= bind(j).to_matrix();
            above.w_axis.truncate()
        })
        .collect();
    // in the world, so a scaled character's springs rest at its own lengths
    let rest = (0..=n).map(|j| if j == 0 { 0.0 } else { p[j].distance(p[j - 1]) }).collect();
    Run { v: vec![Vec3::ZERO; n + 1], rot: (0..n).map(|j| bind(j).rotation).collect(), p, rest, last: r }
}

/// One call of the game's chain step: `steps` 60 Hz steps, the root frame moving to `r` and the colliders from
/// `(was, now)` across them; `whole`: every mass carried all the way.
fn call(c: &Chain, bind: &dyn Fn(usize) -> Transform, u: &mut Run, r: Mat4, (was, now): (&[Mat4], &[Mat4]), steps: usize, whole: bool) {
    let n = c.joints.len();
    let w = |j: usize| &c.words[j.min(n - 1)];
    let moved = r * u.last.inverse();
    u.last = r;
    let (start, target) = (u.p[0], r.transform_point3(bind(0).translation));
    let share = steps as f32;
    // each mass is dragged along by a share of its frame's motion; all of it at once when whole (carried mass by mass
    // through the step, each was pulled at by its neighbour's old place)
    if whole {
        u.p[1..].iter_mut().for_each(|p| *p = moved.transform_point3(*p));
    }
    let carry: Vec<Vec3> = u.p.iter().enumerate().map(|(j, &p)| if whole { Vec3::ZERO } else { (moved.transform_point3(p) - p) * w(j)[CARRY] / share }).collect();
    let mut force = vec![Vec3::ZERO; n + 1];
    for i in 1..=steps {
        u.p[0] = start.lerp(target, i as f32 / share);
        u.p[0] += u.v[0] * DT;
        for j in 1..=n {
            u.p[j] += carry[j];
            let wj = w(j);
            let mut f = Vec3::NEG_Y * wj[GRAVITY] * wj[MASS];
            // length springs to the joint above and (not from the tip) below; only this joint is pushed
            for (k, len) in std::iter::once((j - 1, u.rest[j])).chain((j < n).then(|| (j + 1, u.rest[j + 1]))) {
                let d = u.p[j] - u.p[k];
                let l = d.length();
                if l > 0.0 {
                    f -= (wj[STIFF] * (l - len) + wj[DAMP] * (u.v[j] - u.v[k]).dot(d) / l) * d / l;
                }
            }
            force[j] = f;
        }
        for (j, f) in force.iter().enumerate().skip(1).take(n) {
            let wj = w(j);
            u.v[j] += *f / wj[MASS] * DT;
            u.p[j] += u.v[j] * DT;
            u.v[j] *= wj[KEEP];
        }
        // each mass pushed onto the surface of any ellipsoid it ends up in (placed part way through this frame's move),
        // bounced off it and slid along it
        let shapes: Vec<(Mat4, Mat4)> = was.iter().zip(now).map(|(&a, &b)| a + (b - a) * (i as f32 / share)).map(|m| (m, m.inverse())).collect();
        for j in 1..=n {
            let wj = w(j);
            for (m, inv) in &shapes {
                let l = inv.transform_point3(u.p[j]);
                if l.length_squared() >= 1.0 {
                    continue;
                }
                let l = l.normalize_or(Vec3::Y);
                u.p[j] = m.transform_point3(l);
                let normal = (Mat3::from_mat4(*inv).transpose() * l).normalize();
                let vn = u.v[j].dot(normal) * normal;
                u.v[j] = -wj[BOUNCE] * vn + wj[SLIDE] * (u.v[j] - vn);
            }
        }
        pose(c, bind, u, r);
    }
}

/// The pose pass: each bone turned toward the next mass within its limit, root to tip, and the masses moved onto
/// the turned bones, some of the move fed back into their velocity.
fn pose(c: &Chain, bind: &dyn Fn(usize) -> Transform, u: &mut Run, r: Mat4) {
    let n = c.joints.len();
    let mut above = r;
    for j in 0..=n {
        let a = bind(j);
        let wj = &c.words[j.min(n - 1)];
        let rotation = if j < n {
            // in the parent's space: where the bind pose points the bone, and where the next mass is from its origin
            let posed = (a.rotation * bind(j + 1).translation).normalize_or_zero();
            let seen = (above.inverse().transform_point3(u.p[j + 1]) - a.translation).normalize_or(posed);
            let angle = posed.dot(seen).clamp(-1.0, 1.0).acos().min(wj[LIMIT].to_radians());
            let axis = posed.cross(seen).try_normalize().unwrap_or(Vec3::Z);
            u.rot[j] = Quat::from_axis_angle(axis, angle) * a.rotation;
            u.rot[j]
        } else {
            a.rotation
        };
        let p = above.transform_point3(a.translation);
        u.v[j] += (p - u.p[j]) / DT * wj[KEEP] * wj[FEEDBACK];
        u.p[j] = p;
        above *= Transform { rotation, ..a }.to_matrix();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sophie's pigtail (L_kami01..10): limit, carry, keep from Out of Bounds; mass and feedback the defaults.
    const KAMI: [(f32, f32, f32, f32, f32); 10] = [
        (5.0, 0.7, 0.5, 0.83, 0.2),
        (30.0, 0.5, 0.5, 0.83, 0.2),
        (20.0, 0.4, 0.5, 0.73, 0.2),
        (20.0, 0.3, 0.5, 0.8, 0.2),
        (20.0, 0.3, 0.5, 0.83, 0.2),
        (20.0, 0.5, 0.5, 0.83, 0.2),
        (20.0, 0.2, 0.5, 0.83, 0.2),
        (5.0, 0.7, 0.5, 0.83, 0.2),
        (0.0, 0.7, 0.5, 0.83, 0.2),
        (0.0, 0.7, 0.5, 0.83, 0.2),
    ];

    /// The pigtail hung off a head at `head(frame)` (joint 0 the head, 1.. the chain, 4 cm links posed out sideways),
    /// one step a frame, frame `cut` after a cut; returns each frame's bone rotations.
    fn pigtail(frames: usize, cut: usize, head: impl Fn(usize) -> Mat4) -> Vec<Vec<Quat>> {
        let words = KAMI.map(|(limit, carry, mass, keep, feedback)| {
            let mut w = DEFAULT;
            (w[LIMIT], w[CARRY], w[MASS], w[KEEP], w[FEEDBACK]) = (limit, carry, mass, keep, feedback);
            w
        });
        let chain = Chain { joints: (1..=KAMI.len()).collect(), words: words.to_vec() };
        let parents: Vec<Option<usize>> = (0..=KAMI.len()).map(|k| k.checked_sub(1)).collect();
        let rest: Vec<Transform> = (0..=KAMI.len()).map(|k| Transform::from_xyz(0.04 * (k > 0) as u8 as f32, 0.0, 0.0)).collect();
        let mut runs = Vec::new();
        (0..frames)
            .map(|f| {
                let mut locals = rest.clone();
                solve(std::slice::from_ref(&chain), &[], &parents, &rest, head(f), &mut locals, (&mut runs, &mut Vec::new()), 1, f == cut);
                locals.iter().map(|t| t.rotation).collect()
            })
            .collect()
    }

    /// Each frame's largest angle between two runs' bones.
    fn apart(a: &[Vec<Quat>], b: &[Vec<Quat>]) -> Vec<f32> {
        a.iter().zip(b).map(|(a, b)| a.iter().zip(b).map(|(x, y)| x.angle_between(*y)).fold(0f32, f32::max)).collect()
    }

    #[test]
    fn hangs_within_its_limits_and_settles() {
        // posed straight out sideways, the pigtail droops under gravity no further than its cones allow
        let off = apart(&pigtail(300, usize::MAX, |_| Mat4::IDENTITY), &vec![vec![Quat::IDENTITY; KAMI.len() + 1]; 300]);
        assert!(off[0] > 1f32.to_radians() && off.iter().all(|&o| o <= 30.001f32.to_radians()), "{:?}", off[0].to_degrees());
        assert!((off[299] - off[200]).abs() < 1e-4, "{} {}", off[200].to_degrees(), off[299].to_degrees());
    }

    #[test]
    fn a_jump_swings_then_settles_back() {
        let still = pigtail(300, usize::MAX, |_| Mat4::IDENTITY);
        let jump = pigtail(300, usize::MAX, |f| Mat4::from_translation(Vec3::Z * 0.03 * f.min(10) as f32));
        let apart = apart(&still, &jump);
        assert!(apart[..20].iter().any(|&d| d > 2f32.to_radians()), "{:?}", &apart[..20]);
        assert!(apart[299] < 0.1f32.to_radians(), "{}", apart[299].to_degrees());
    }

    #[test]
    fn a_cut_carries_the_hair_whole() {
        // the head jumps 30 cm in a frame: after a cut the hair hangs as before, else it whips
        let still = pigtail(300, usize::MAX, |_| Mat4::IDENTITY);
        let head = |f: usize| Mat4::from_translation(Vec3::Z * 0.3 * (f >= 200) as u8 as f32);
        let whip = apart(&still, &pigtail(300, usize::MAX, head));
        let cut = apart(&still, &pigtail(300, 200, head));
        assert!(whip[200..].iter().any(|&d| d > 10f32.to_radians()), "{:?}", &whip[200..220]);
        assert!(cut.iter().all(|&d| d < 0.5f32.to_radians()), "{:?}", &cut[200..220]);
    }

    #[test]
    fn a_mirrored_head_mirrors_the_hair() {
        // a left-hander's root has scale x −1: the solved hair must be the mirror image of the right-hander's
        let place = |hand: f32, f: usize| Mat4::from_scale(Vec3::new(hand, 1.0, 1.0)) * Mat4::from_translation(Vec3::Z * 0.03 * f.min(10) as f32);
        let off = apart(&pigtail(60, usize::MAX, |f| place(1.0, f)), &pigtail(60, usize::MAX, |f| place(-1.0, f)));
        assert!(off.iter().all(|&o| o < 1e-3), "{off:?}");
    }
}

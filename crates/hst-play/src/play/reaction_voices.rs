//! The reaction voices after a point (`hst_sim::sound::reaction_voice`): on the reactions' 5th frame each player
//! the post-point camera has in close view (`sound::reaction_view`) cheers or groans by their reaction motion. The
//! view test takes the model's origin, its pelvis as last posed, and the player's spot.

use bevy::prelude::*;
use hst_sim::sound;

use super::Game;

pub fn plugin(app: &mut App) {
    app.init_resource::<Frames>().add_systems(FixedUpdate, step.after(super::simulate).before(super::start_effects));
}

/// Frames since the reactions started.
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Frames(u32);

fn step(mut g: ResMut<Game>, mut frames: ResMut<Frames>, posed: Option<Res<super::bodyhit::Posed>>, bones: Option<Res<super::bodyhit::Bones>>) {
    let frames = &mut frames.0;
    if !g.post.as_ref().is_some_and(|p| p.reacted) {
        return *frames = 0;
    }
    *frames += 1;
    if *frames != 5 || g.players.len() < 2 {
        return;
    }
    let g = &mut *g;
    let v = g.cam.view;
    let [x, y, z] = v.eye;
    let eye = [x, y, z, 1.0];
    let [x, y, z] = v.rot[2];
    let look = [x, y, z, 0.0];
    let view = matrix(&v);
    let focal = 240.0 / hst_sim::musl::tanf(v.fov);
    let n = g.players.len() as u32;
    for i in 0..g.players.len() {
        let p = &g.players[i];
        // ponytail: character 13 tests another spot of its own (no recording has it); every other character's is its position
        let at = [p.pos[0], p.pos[1], p.pos[2], 1.0];
        let origin = pelvis(&g, i, posed.as_deref(), bones.as_deref()).unwrap_or(at);
        if !sound::reaction_view(eye, look, &view, focal, origin, at) {
            continue;
        }
        // ponytail: a body-hit player standing has no reaction motion; any other motion voices alike
        let motion = p.root.map_or(-1, |r| r.motion as i32);
        let lost = i as i32 & 1 != g.post_winner & 1;
        let Some(rv) = sound::reaction_voice(n, motion, lost, false, || g.rng.shared.r15() % 100) else { continue };
        if let Some(play) = g.voices[i].react(i, rv, || g.rng.shared.r15()) {
            g.lines.push(super::emote::Line::Reaction(i, play));
        }
    }
}

/// Player `i`'s `Bip01Pelvis` in game space as the motion player last posed it (`step` runs before this tick's
/// pose), or `None` before the first pose.
fn pelvis(g: &Game, i: usize, posed: Option<&super::bodyhit::Posed>, bones: Option<&super::bodyhit::Bones>) -> Option<[f32; 4]> {
    let data = g.data.get(i)?;
    let n = bones?.of(i, data).at[4]?;
    let w = hst_sim::pose::node_world(&data.skeleton, posed?.locals(i)?, n, &super::player_matrix(&g.players[i]));
    Some([w[3][0], w[3][1], w[3][2], 1.0])
}

/// `v`'s world → camera matrix as `vu0::transform` takes it (column k is camera axis k).
pub(super) fn matrix(v: &hst_sim::camera::View) -> [[f32; 4]; 4] {
    let mut m = [[0.0; 4]; 4];
    for (k, axis) in v.rot.iter().enumerate() {
        for (row, &a) in m.iter_mut().zip(axis) {
            row[k] = a;
        }
        m[3][k] = -axis.iter().zip(v.eye).map(|(a, e)| a * e).sum::<f32>();
    }
    m[3][3] = 1.0;
    m
}

impl super::snapshot::Wire for Frames {}

#[cfg(test)]
mod tests {
    #[test]
    fn matrix_is_local() {
        let v = hst_sim::camera::View { rot: [[0.6, 0.0, -0.8], [0.0, 1.0, 0.0], [0.8, 0.0, 0.6]], eye: [1.0, -2.0, 3.0], fov: 0.5 };
        let p = [4.0, 5.0, -6.0];
        let got = hst_sim::vu0::transform(&super::matrix(&v), [p[0], p[1], p[2], 1.0]);
        let want = v.local(p);
        assert!((0..3).all(|k| (got[k] - want[k]).abs() < 1e-5), "{got:?} {want:?}");
    }
}

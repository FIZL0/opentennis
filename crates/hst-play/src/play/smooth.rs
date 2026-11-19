//! B735: rollback corrections glide on screen. When a rollback re-runs ticks on a peer's real pads and the match
//! comes out somewhere else than it was drawn, the drawn ball, the other machines' players and the red landing
//! marker are held where they were drawn and slide to the corrected place over [`GLIDE`] instead of jumping. This
//! machine's own players are never moved (their controls stay instant), and a big change (a hit one run had and the
//! other didn't, the point over, a jump past [`SNAP_BALL`] / [`SNAP_PLAYER`]) is drawn at once.
//!
//! Draw only: [`Smooth`] is an offset added to the drawn transforms, never saved in a snapshot, never read by a tick,
//! so the match, its RNG and its state hashes are the same with or without it.

use bevy::prelude::*;

use super::{BallView, Figure, Game};

/// How long a correction takes to glide out (5 frames at 60).
pub const GLIDE: f32 = 5.0 / 60.0;
/// A ball correction this long (metres) or more is drawn at once: sliding it would show the ball crossing the court.
pub const SNAP_BALL: f32 = 3.0;
/// A player (or the red marker) moved this far by a correction is drawn there at once.
pub const SNAP_PLAYER: f32 = 2.0;

/// What's drawn on top of the match: each offset, game space, from where the match is to where it's drawn.
#[derive(Resource, Default, Clone, Debug)]
pub struct Smooth {
    pub ball: Vec3,
    pub players: [Vec3; 4],
    pub red: Vec2,
    /// Seconds left of the glide; the offsets shrink to nothing over it.
    pub left: f32,
}

impl Smooth {
    /// Where the ball is drawn for a match ball at `pos`.
    #[cfg(test)]
    pub fn ball_at(&self, pos: super::V3) -> super::V3 {
        (Vec3::from(pos) + self.ball).to_array()
    }
}

/// What a correction compares: the drawn things where the match had them before a rollback, and after it is back at
/// the same tick.
#[derive(Clone, Debug)]
pub struct Seen {
    /// The tick the rollback's re-runs come back to.
    pub frame: usize,
    ball: Vec3,
    vel: Vec3,
    shots: i32,
    /// The ball's in play (serve or rally): a point starts and ends between two that differ.
    live: bool,
    players: Vec<Vec3>,
    red: Option<Vec2>,
}

impl Seen {
    pub fn of(g: &Game, frame: usize) -> Self {
        Seen {
            frame,
            ball: g.flight.ball.pos.into(),
            vel: g.flight.ball.vel.into(),
            shots: g.shots,
            live: matches!(g.phase, super::Phase::Serve | super::Phase::Rally),
            players: g.players.iter().map(|p| Vec3::from(p.pos)).collect(),
            red: g.marks.red.or(g.marks.serve_red).map(Vec2::from),
        }
    }
}

/// Player `i` of `n`'s seat (as `Pads::slot_of`).
fn seat(i: usize, n: usize) -> usize {
    if n == 4 { [0, 2, 1, 3][i] } else { i }
}

/// A rollback is back at `before.frame` with the match now `after`: what was drawn stays put and glides from there.
/// `own` are this machine's seats (never moved).
pub fn correct(s: &mut Smooth, before: &Seen, after: &Seen, own: &[usize]) {
    // the point over (or the next one set up) in one run only: everyone's put somewhere else
    let same_point = before.live == after.live;
    // a hit one run had and the other didn't (or not the same number of them): the ball's somewhere else for real
    let same_shot = same_point && before.shots == after.shots && before.vel.xz().dot(after.vel.xz()) >= 0.0;
    let mut moved = false;
    let d = before.ball - after.ball;
    if same_shot && (s.ball + d).length() < SNAP_BALL {
        s.ball += d;
        moved |= d != Vec3::ZERO;
    } else {
        s.ball = Vec3::ZERO;
    }
    let n = after.players.len();
    for (i, (b, a)) in before.players.iter().zip(&after.players).enumerate().take(4) {
        if own.contains(&seat(i, n)) {
            s.players[i] = Vec3::ZERO;
            continue;
        }
        let d = b - a;
        if same_point && (s.players[i] + d).length() < SNAP_PLAYER {
            s.players[i] += d;
            moved |= d != Vec3::ZERO;
        } else {
            s.players[i] = Vec3::ZERO;
        }
    }
    match (before.red, after.red) {
        (Some(b), Some(a)) if same_point && (s.red + b - a).length() < SNAP_PLAYER => {
            s.red += b - a;
            moved |= b != a;
        }
        _ => s.red = Vec2::ZERO,
    }
    if moved {
        s.left = GLIDE;
    }
}

/// One drawn frame `dt` seconds long: the offsets shrink linearly to nothing at the glide's end.
pub fn fade(s: &mut Smooth, dt: f32) {
    if s.left <= 0.0 {
        *s = Smooth::default();
        return;
    }
    let keep = (1.0 - dt / s.left).max(0.0);
    s.ball *= keep;
    s.players.iter_mut().for_each(|p| *p *= keep);
    s.red *= keep;
    s.left -= dt;
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Smooth>()
        .add_systems(Update, (fade_frame.before(super::draw), shift.after(super::draw).before(crate::character::animate)))
        .add_systems(Update, marker.after(crate::effects::draw_marks));
}

fn fade_frame(mut s: ResMut<Smooth>, time: Res<Time>) {
    if s.left > 0.0 || s.ball != Vec3::ZERO || s.red != Vec2::ZERO || s.players.iter().any(|p| *p != Vec3::ZERO) {
        fade(&mut s, time.delta_secs());
    }
}

/// The drawn ball and players moved by their offsets (`draw` placed them where the match has them).
fn shift(s: Res<Smooth>, mut figures: Query<(&Figure, &mut Transform)>, mut ball: Query<(&BallView, &mut Transform), Without<Figure>>) {
    if s.left <= 0.0 {
        return;
    }
    for (f, mut t) in &mut figures {
        if let Some(d) = s.players.get(f.0) {
            t.translation += *d;
        }
    }
    for (v, mut t) in &mut ball {
        // the shadow and outline: `ball_shadow::place`
        if !v.0 {
            t.translation += s.ball;
        }
    }
}

/// The red landing marker moved by its offset.
fn marker(s: Res<Smooth>, marks: Option<Res<crate::effects::LandingMarks>>, mut q: Query<&mut Transform>) {
    let Some(marks) = marks.filter(|m| m.red_at.is_some() && s.left > 0.0) else { return };
    if let Ok(mut t) = q.get_mut(marks.red_root()) {
        t.translation.x += s.red.x;
        t.translation.z += s.red.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(ball: [f32; 3], players: &[[f32; 3]]) -> Seen {
        Seen {
            frame: 0,
            ball: ball.into(),
            vel: Vec3::new(0.0, 0.0, 0.3),
            shots: 1,
            live: true,
            players: players.iter().map(|&p| p.into()).collect(),
            red: Some(Vec2::new(1.0, 5.0)),
        }
    }

    /// A small correction is held where it was drawn and glides out in 5 frames; this machine's own players never
    /// move; a long one snaps.
    #[test]
    fn corrections_glide_own_players_stay() {
        let mut s = Smooth::default();
        let p = [[0.0, 0.0, -10.0], [0.0, 0.0, 10.0]];
        let mut b = seen([1.0, 1.0, 0.0], &p);
        let mut a = seen([1.0, 1.0, 0.5], &[[0.0, 0.0, -10.0], [0.5, 0.0, 10.0]]);
        a.players[0].x = 0.4;
        a.red = Some(Vec2::new(1.5, 5.0));
        correct(&mut s, &b, &a, &[0]);
        assert_eq!(s.ball, Vec3::new(0.0, 0.0, -0.5));
        assert_eq!(s.players[0], Vec3::ZERO, "own player");
        assert_eq!(s.players[1], Vec3::new(-0.5, 0.0, 0.0));
        assert_eq!(s.red, Vec2::new(-0.5, 0.0));
        let mut sizes = vec![];
        for _ in 0..6 {
            fade(&mut s, 1.0 / 60.0);
            sizes.push(s.ball.length());
        }
        assert!(sizes.windows(2).all(|w| w[1] <= w[0]), "{sizes:?}");
        assert!(sizes[3] > 0.0 && sizes[4] < 1e-6, "{sizes:?}");
        // the ball far off, or a hit only one run had: drawn at once
        b.ball.z = 4.0;
        correct(&mut s, &b, &a, &[0]);
        assert_eq!(s.ball, Vec3::ZERO);
        b.ball.z = 0.0;
        b.vel.z = -0.3;
        correct(&mut s, &b, &a, &[0]);
        assert_eq!(s.ball, Vec3::ZERO);
    }
}

//! The ball's bounce effects around `start_effects`: each frame before it, the point set-up (rain, the smash burst
//! stops showing) and the body hit; after it and the footstep effects (the game runs feet, hit sparks, then this),
//! the bounce debris, which draws the smash burst's clods from the sound manager's generator.

use bevy::prelude::*;

use super::{Game, Phase};
use crate::effects::BallBounce;
use crate::weather::Weather;

pub(super) fn set_up(fx: Option<ResMut<BallBounce>>, g: Res<Game>, w: Option<Res<Weather>>) {
    let Some(mut fx) = fx else { return };
    let wet = w.is_some_and(|w| hst_sim::weather::rain(w.today().weather));
    fx.set_up(g.phase == Phase::Serve, wet, g.body_hit.is_some());
}

pub(super) fn throw(fx: Option<ResMut<BallBounce>>, mut g: ResMut<Game>) {
    let Some(mut fx) = fx else { return };
    let f = &g.flight;
    let smash = g.smashed && hst_sim::sound::kmh(f.ball.vel) >= 85.0;
    let ball = f.ball.pos;
    let mt = &mut g.rng.sound;
    fx.debris(ball, smash, &mut || mt.next());
}

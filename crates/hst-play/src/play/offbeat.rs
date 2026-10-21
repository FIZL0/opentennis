//! B40g2 (concept, `concept/offbeat-rules`): Offbeat Rules, the confirm screen's match rules
//! (`--offbeat irregular,slow,turbo,ghost`, any of them). The first two are the original's:
//! - **Irregular bounce** (its rule 1): every bounce of a rally turns the ball by the shot's two launch uniforms,
//!   0°..15° flatter and ±15° (±20° for a lob) sideways (`hst_sim::ball::irregular`). The CPUs' predicted path runs
//!   the same flight, so they see it coming. The ball is drawn as `ball_irregular.mdl` and the court's bounce
//!   sound is key 13, never slowed (`hst_sim::sound::Bounces::irregular`).
//! - **Slow motion** (its rule 2): from the serve's set-up to the point's end, every odd tick of the point is
//!   followed by one in-between frame, about ⅔ speed (the instant replay's pacing, `replay_pace`).
//!
//! and two of ours (the round-1 prototypes of the journal's proposal):
//! - **Turbo rally**: the rally speeds up 10 % a shot after the serve's return, up to ×2.
//! - **Ghost ball**: the ball, its shadow and its trail disappear once it is past the net and stay gone until the
//!   next hit (or the point ends); only its bounce effects show where it went.
//!
//! Only irregular bounce changes the sim; the others change the clock or the picture, so the ticks, inputs and
//! results stay the same. With every rule off the match is the original's.

use bevy::prelude::*;

use super::{BallView, Game, Phase};

/// The match's rules (a resource for setup, then `Game::offbeat`).
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Offbeat {
    pub irregular: bool,
    pub slow: bool,
    pub turbo: bool,
    pub ghost: bool,
}

impl Offbeat {
    /// The flag's names, in the rules page's order.
    pub const NAMES: [&str; 4] = ["irregular", "slow", "turbo", "ghost"];

    pub fn parse(s: &str) -> Self {
        let mut o = Self::default();
        for name in s.split(',').map(str::trim) {
            if let Some(i) = Self::NAMES.iter().position(|n| *n == name) {
                o.set(i, true);
            }
        }
        o
    }

    /// The flag's value, or None with every rule off.
    pub fn flag(&self) -> Option<String> {
        let on: Vec<&str> = Self::NAMES.iter().enumerate().filter(|&(i, _)| self.get(i)).map(|(_, n)| *n).collect();
        (!on.is_empty()).then(|| on.join(","))
    }

    pub fn get(&self, i: usize) -> bool {
        [self.irregular, self.slow, self.turbo, self.ghost][i]
    }

    pub fn set(&mut self, i: usize, on: bool) {
        *[&mut self.irregular, &mut self.slow, &mut self.turbo, &mut self.ghost][i] = on;
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostStartup, start.after(super::setup))
        .add_systems(FixedUpdate, pace.after(super::replay_pace::pace))
        .add_systems(Update, ghost.after(super::draw));
}

/// The match takes its rules; with irregular bounce on, the drawn ball becomes the irregular one.
#[allow(clippy::too_many_arguments)]
fn start(
    mut commands: Commands,
    rules: Option<Res<Offbeat>>,
    game: Option<ResMut<Game>>,
    args: Res<crate::Args>,
    balls: Query<(Entity, &BallView)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(rules), Some(mut g)) = (rules, game) else { return };
    g.offbeat = *rules;
    g.bounces.irregular = rules.irregular;
    if !rules.irregular {
        return;
    }
    let Some(game_xb) = hst_data::iso::Iso::open(&args.iso).ok().and_then(|mut iso| iso.read("CMN/GAME.XB").ok()) else { return };
    let parts: Vec<_> = crate::models(&game_xb, |n| n.ends_with("ball_irregular.mdl"), &mut images).into_iter().flat_map(|(_, p)| p).collect();
    if parts.is_empty() {
        return warn!("offbeat: no ball_irregular.mdl on the disc");
    }
    for (e, _) in balls.iter().filter(|(_, v)| !v.0) {
        commands.entity(e).despawn_related::<Children>();
        for (mesh, material) in &parts {
            commands.entity(e).with_child((Mesh3d(meshes.add(mesh.clone())), MeshMaterial3d(materials.add(material.clone()))));
        }
    }
}

/// The ball launch's two uniforms from the shared generator (every launch draws them), for a shot of `g.shot`'s
/// kind: the turn they give its bounces under irregular bounce (`Flight::irregular`), else None.
pub(super) fn launch(g: &mut Game) -> Option<[f32; 2]> {
    let pitch = g.rng.shared.unit();
    let yaw = g.rng.shared.unit();
    g.offbeat.irregular.then(|| hst_sim::ball::irregular(g.shot.kind, pitch, yaw))
}

/// Turbo rally's step per shot after the serve's return, and its top speed.
const TURBO_STEP: f32 = 0.1;
const TURBO_TOP: f32 = 2.0;

/// The clock's speed for the next tick (1 = the match's own): slow motion halves it after the point's odd ticks
/// (one in-between frame), turbo rally raises it with the rally's shots; both together multiply.
pub fn speed(g: &Game) -> f32 {
    let o = &g.offbeat;
    let slow = o.slow && matches!(g.phase, Phase::Serve | Phase::Rally) && g.instant.ticks % 2 == 1;
    let turbo = match g.shots {
        n if o.turbo && g.phase == Phase::Rally && n > 2 => (1.0 + TURBO_STEP * (n - 2) as f32).min(TURBO_TOP),
        _ => 1.0,
    };
    if slow { 0.5 * turbo } else { turbo }
}

/// End of each tick, after the replay's pacing (which owns the clock during a replay and puts it back to 1 after).
fn pace(g: Option<Res<Game>>, mut virt: ResMut<Time<Virtual>>) {
    let Some(g) = g else { return };
    if g.instant.replaying {
        return;
    }
    let s = speed(&g);
    if virt.relative_speed() != s {
        virt.set_relative_speed(s);
    }
}

/// Past this distance beyond the net (m) the ghost ball is gone.
const GHOST_PAST: f32 = 1.0;

/// Ghost ball: the ball is out of sight, past the net on the far side from its hitter, until the next hit or the
/// point's end; only its bounce effects show where it went.
pub fn ghosted(g: &Game) -> bool {
    let z = g.flight.ball.pos[2];
    g.offbeat.ghost
        && g.phase == Phase::Rally
        && g.shots > 0
        && g.players.get(g.last_hitter as usize).is_some_and(|p| p.pos[2] * z < 0.0 && z.abs() > GHOST_PAST)
}

/// Hide the ball, its shadow and its outline while it is a ghost; only on a change, so the other hidings
/// (`emote::hide_ball`, the ceremony's) stay.
fn ghost(g: Option<Res<Game>>, mut q: Query<&mut Visibility, Or<(With<BallView>, With<crate::shade::Ball>)>>, mut hidden: Local<bool>) {
    let Some(g) = g else { return };
    let hide = ghosted(&g);
    if std::mem::replace(&mut *hidden, hide) == hide {
        return;
    }
    for mut v in &mut q {
        *v = if hide { Visibility::Hidden } else { Visibility::Inherited };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_round_trips() {
        assert_eq!(Offbeat::parse("").flag(), None);
        let o = Offbeat::parse("slow, ghost,nonsense");
        assert_eq!(o, Offbeat { slow: true, ghost: true, ..default() });
        assert_eq!(o.flag().as_deref(), Some("slow,ghost"));
        assert_eq!(Offbeat::parse("irregular,slow,turbo,ghost").flag().as_deref(), Some("irregular,slow,turbo,ghost"));
    }

    /// A doubles match of computer players with every round-1 rule but irregular bounce: the ticks are the same
    /// match as with the rules off (they only pace the clock and hide the ball), slow motion halves the speed after
    /// each odd tick of a point, turbo builds up within a rally, and the ball goes out of sight past the net.
    #[test]
    fn pacing_and_ghost_leave_the_match_alone() {
        use super::super::tick_input::{IDLE, Replay};
        use super::super::*;
        use crate::{Args, GameSpace};
        if !hst_data::fixture::has_iso() {
            return;
        }
        let set_up = |rules: Offbeat| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default()));
            app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
            let iso = hst_data::fixture::ISO_PATH.into();
            app.insert_resource(Args { iso, archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: false, chars: vec![6, 1, 2, 3], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
            app.insert_resource(Pads { seats: Some([false; 4]), autoplay: false, ..default() });
            app.insert_resource(Replay(Default::default()));
            app.init_resource::<super::super::tick_input::TickPads>();
            app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
            app.add_systems(Startup, setup);
            app.add_systems(FixedUpdate, (remember, super::super::tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
            app.update();
            app.world_mut().resource_mut::<Game>().offbeat = rules;
            app
        };
        let mut off = set_up(Offbeat::default());
        let mut on = set_up(Offbeat { slow: true, turbo: true, ghost: true, ..default() });
        let (mut halved, mut whole, mut fast, mut ghosts) = (0, 0, 0, 0);
        for _ in 0..4000 {
            for app in [&mut off, &mut on] {
                app.world_mut().resource_mut::<Replay>().0.push_back([IDLE; 4]);
                app.world_mut().run_schedule(FixedUpdate);
            }
            let g = on.world().resource::<Game>();
            let s = speed(g);
            let pointing = matches!(g.phase, Phase::Serve | Phase::Rally);
            match (pointing, g.instant.ticks % 2 == 1) {
                (true, true) => assert!((0.5..=0.5 * TURBO_TOP).contains(&s), "after an odd tick: halved ({s})"),
                (true, false) => assert!(s >= 1.0, "after an even tick: whole ({s})"),
                (false, _) => assert_eq!(s, 1.0, "between points ({:?})", g.phase),
            }
            halved += (s < 1.0) as u32;
            whole += (s == 1.0) as u32;
            fast += (s > 1.0) as u32;
            ghosts += ghosted(g) as u32;
            assert_eq!(speed(off.world().resource::<Game>()), 1.0, "rules off: the match's own speed");
        }
        let plain = |app: &App| {
            let mut g = app.world().resource::<Game>().clone();
            g.offbeat = default();
            format!("{g:?}")
        };
        assert!(plain(&off) == plain(&on), "the rules changed the match");
        assert!(halved > 500 && whole > 500 && fast > 50 && ghosts > 20, "{halved} {whole} {fast} {ghosts}");
    }
}

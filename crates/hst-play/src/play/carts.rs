//! P134: the court's moving structures in the match (court 5's ball-boy carts turn their wheels, `hst_sim::court::
//! Motions`, P133). The original steps them once per court draw pass: every displayed frame of the court (the intro,
//! the cut-aways and the instant replays count), but not on the first pass after the court is built, not while
//! paused, not on slow motion's in-between frames, and not for a cart the pass's eye test hides. The match does it
//! once per match tick at the end of the tick, after the cameras have stepped (no tick runs while paused, and slow
//! motion only adds in-between frames between ticks), and once per intro tick (`intro`). The ball's sweeps read
//! `Game::world`, whose carts' nodes the last pass posed: the update sees the pose of the frame drawn before it.
//!
//! The clocks are per-tick state (`Game::carts`: hashed, kept by snapshots and rollback, sent on a resync); the
//! world's poses follow from them. An instant replay puts the match back to its serve but not the carts: the
//! original's draw passes go on through it, so its clocks do too.

use std::fmt;

use bevy::prelude::*;
use hst_data::layout::category::STRUCTURE;
use hst_sim::court::{Motions, Phase, PropMotion};
use hst_sim::mesh::World;

use super::Game;

/// The court's structure motions (none off court 5 or under a court mod).
#[derive(Clone, Default)]
pub(super) struct Carts {
    motions: Motions,
    /// A resync's phase until `adopt` binds it to this machine's motions.
    wire: Option<Phase>,
}

impl Carts {
    pub(super) fn new(motions: Motions) -> Carts {
        Carts { motions, wire: None }
    }

    /// One court draw pass over `world`: the first after the build steps nothing; the props in `held` don't step.
    pub(super) fn pass(&mut self, world: &mut World, held: &[usize]) {
        self.motions.pass(world, true, |p| held.contains(&p));
    }

    /// The passes' phase so far.
    #[cfg(test)]
    pub(super) fn phase(&self) -> Phase {
        self.motions.phase()
    }

    /// A resync's carts (`wire` only) take this machine's motions at the sent phase, posing `world`.
    pub(super) fn adopt(&mut self, local: &Carts, world: &mut World) {
        self.motions = local.motions.clone();
        if let Some(p) = self.wire.take() {
            self.motions.set_phase(&p, world);
        }
    }
}

impl fmt::Debug for Carts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.motions.phase().fmt(f)
    }
}

impl serde::Serialize for Carts {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.motions.phase().serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for Carts {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Phase::deserialize(d).map(|p| Carts { motions: Motions::default(), wire: Some(p) })
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(FixedUpdate, tick.after(super::start_effects).before(super::play_sounds));
}

/// The pass drawn after a match tick, from the shared camera's view (the match camera, a cut-away or the replay
/// camera; never a player's own camera, which the original doesn't have and the other machines don't share): a cart
/// the eye test would hide (`net_hide::buried`) doesn't step.
fn tick(mut g: ResMut<Game>) {
    let g = &mut *g;
    if g.carts.motions.list.is_empty() {
        return;
    }
    let (view, w) = (g.cam.view, &g.world.0);
    let hidden = |m: &&PropMotion| super::net_hide::tested(STRUCTURE, m.code) && super::net_hide::buried(&view, &w.models, &w.props[m.prop]);
    let held: Vec<usize> = g.carts.motions.list.iter().filter(hidden).map(|m| m.prop).collect();
    g.carts.pass(&mut g.world.make_mut().0, &held);
}

/// The pass drawn on an intro tick: its flyover hides nothing, and the match camera it holds after isn't in a cart.
pub(super) fn intro(mut g: ResMut<Game>) {
    let g = &mut *g;
    if g.carts.motions.list.is_empty() {
        return;
    }
    g.carts.pass(&mut g.world.make_mut().0, &[]);
}

/// The carts as they are, for an instant replay to keep across putting the match back to its serve.
pub(super) fn keep(w: &bevy::prelude::World) -> Option<(Carts, super::snapshot::Const<(World, Vec<hst_sim::ball::Material>)>)> {
    w.get_resource::<Game>().map(|g| (g.carts.clone(), g.world.clone()))
}

pub(super) fn put_back(w: &mut bevy::prelude::World, kept: Option<(Carts, super::snapshot::Const<(World, Vec<hst_sim::ball::Material>)>)>) {
    if let (Some((carts, world)), Some(mut g)) = (kept, w.get_resource_mut::<Game>()) {
        (g.carts, g.world) = (carts, world);
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{Args, GameSpace};

    /// A bot singles match on disc court 5 (the carts'), only the setup and the fixed schedule.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: Some(5), play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 1, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.add_plugins(super::plugin);
        app.update();
        app
    }

    fn tick(app: &mut App) {
        app.world_mut().run_schedule(FixedUpdate);
    }

    /// Every cart's prop in the match's world is posed as its motion has it.
    fn posed(g: &Game) {
        for m in &g.carts.motions.list {
            assert_eq!(g.world.0.props[m.prop].nodes, m.world_nodes(), "prop {}", m.prop);
        }
    }

    /// P134: court 5's four carts step once a match tick but the first (the court's first draw pass), wrap at their
    /// motion's 40 frames, and the ball's world carries their pose; a snapshot and a resync put both back.
    #[test]
    fn court_5_carts_turn_once_a_tick() {
        if !hst_data::fixture::has_iso() {
            hst_data::fixture::skip(format_args!("disc missing, skipped"));
            return;
        }
        let mut app = app();
        let g = app.world().resource::<Game>();
        let start = g.carts.phase();
        assert!(start.first && start.clocks.len() == 4 && start.clocks.iter().all(|c| c.1 && c.0.time == 0.0), "{start:?}");
        let disc: Vec<_> = g.world.0.props.iter().map(|p| p.nodes.clone()).collect();
        let ticks = 130;
        let mut kept = None;
        for n in 1..=ticks {
            tick(&mut app);
            if n == 70 {
                kept = Some((super::super::snapshot::save(app.world_mut()), app.world().resource::<Game>().carts.phase()));
            }
        }
        let g = app.world().resource::<Game>();
        let p = g.carts.phase();
        // the first pass held them: ticks − 1 steps, sampled one behind the time
        let sampled = ((ticks - 2) % 40) as f32;
        assert!(!p.first && p.clocks.iter().all(|&(c, again)| !again && c.sampled == sampled && c.time == sampled + 1.0), "{p:?}");
        posed(g);
        let moved: Vec<_> = g.world.0.props.iter().zip(&disc).enumerate().filter(|(_, (p, d))| p.nodes != **d).map(|(i, _)| i).collect();
        assert_eq!(moved, g.carts.motions.list.iter().map(|m| m.prop).collect::<Vec<_>>(), "only the carts move");

        // a resync's bytes come back at the same phase, the world posed to it
        let bytes = super::super::snapshot::encode(app.world_mut());
        let (s, at) = kept.unwrap();
        super::super::snapshot::restore(app.world_mut(), &s);
        assert_eq!(app.world().resource::<Game>().carts.phase(), at);
        posed(app.world().resource::<Game>());
        super::super::snapshot::decode(app.world_mut(), &bytes).unwrap();
        assert_eq!(app.world().resource::<Game>().carts.phase(), p);
        posed(app.world().resource::<Game>());
        tick(&mut app);
        let next = app.world().resource::<Game>().carts.phase();
        assert!(next.clocks.iter().all(|&(c, _)| c.sampled == (sampled + 1.0) % 40.0), "{next:?}");
    }

    /// An instant replay puts the match back to its serve but the carts go on from where they are.
    #[test]
    fn a_replay_keeps_the_carts_going() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let mut app = app();
        for _ in 0..10 {
            tick(&mut app);
        }
        let s = super::super::snapshot::save(app.world_mut());
        for _ in 0..25 {
            tick(&mut app);
        }
        let now = app.world().resource::<Game>().carts.phase();
        let kept = super::keep(app.world());
        super::super::snapshot::restore(app.world_mut(), &s);
        super::put_back(app.world_mut(), kept);
        assert_eq!(app.world().resource::<Game>().carts.phase(), now);
        posed(app.world().resource::<Game>());
    }
}

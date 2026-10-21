//! M7: the original's alternate match camera (`hst_sim::alt_cam`): low behind the one human, switched on and off with
//! the `camera` bind (Select) when exactly one human plays on this machine (never online), gliding over 15 ticks
//! each way, and fading the human toward half transparency while their body hides the ball coming at them.
//!
//! Presentation only: the shared match camera (`Game::cam`, which the cut-aways and reaction voices read) keeps
//! stepping as before and nothing in play reads this one; the picture shows it through `own_camera::shown`. The
//! last choice is kept in `camera.txt` for the next match. Select only switches in live play (`live`): a press
//! while the players can't move (the intro, post-point, cut-aways, change of ends, replays) is dropped.

use bevy::prelude::*;
use hst_sim::alt_cam::{self, Fade, Follow, GLIDE};
use hst_sim::camera::{Camera, Scene, View, swing};

use super::own_camera::{self, OwnCam};
use super::{Figure, Game, Phase, Pads, controls};
use crate::Args;

/// Rally ticks the fade waits before it looks at the ball.
const SETTLE: u32 = 30;

#[derive(Resource, Default)]
pub(super) struct AltCamera {
    /// The alternate camera is chosen.
    on: bool,
    /// The human it follows and the shot following them (made at the first tick it shows).
    follow: Option<(usize, bool, Follow)>,
    /// A switch's glide: ticks left and the view it started from.
    glide: Option<(u32, View)>,
    /// After switching off: a freshly reset match camera, shown until the shared one cuts to the next point.
    normal: Option<Camera>,
    /// The view shown last tick.
    last: Option<View>,
    fade: Fade,
    /// Rally ticks so far.
    rally: u32,
    /// Last tick's phase was the serve's.
    serving: bool,
    /// A Select press waiting for the next tick.
    press: bool,
    /// The ball the shot looks at: the live one, held where the rally left it once the point is over (B628).
    held: [f32; 3],
}

impl AltCamera {
    /// The human's alpha this tick (1 when not faded).
    pub(super) fn alpha(&self) -> f32 {
        self.fade.alpha()
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, load)
        .add_systems(Update, (press.run_if(|w: &World| !super::front_end::paused(w) && !super::intro::running(w)), fade))
        // after every system that writes the shared camera's view this tick, as `own_camera::plain`
        .add_systems(
            FixedUpdate,
            step.after(super::simulate)
                .after(super::cutaway::step)
                .after(super::replay_camera::step)
                .after(super::change_ends::step)
                .after(super::start_effects),
        );
}

/// `camera.txt` in the per-user data folder.
fn path(iso: &str) -> std::path::PathBuf {
    crate::store::path(iso, "camera.txt")
}

fn load(mut commands: Commands, args: Res<Args>) {
    let on = crate::store::read_to_string(path(&args.iso)).is_ok_and(|t| t.trim() == "alternate");
    commands.insert_resource(AltCamera { on, ..default() });
}

fn save(iso: &str, on: bool) {
    // tests never write beside the disc
    if cfg!(test) {
        return;
    }
    if let Err(e) = crate::store::write(path(iso), if on { "alternate\n" } else { "normal\n" }) {
        warn!("camera.txt: {e}");
    }
}

/// The one human playing on this machine, if exactly one does and the match isn't online.
pub(super) fn human(g: &Game, pads: &Pads, online: bool) -> Option<usize> {
    let n = g.players.len();
    let mut humans = (0..n).filter(|&i| pads.slot_of(i, n).is_some());
    let first = humans.next()?;
    (!online && humans.next().is_none()).then_some(first)
}

/// B622: the players can move (the serve or the rally, no cut-away or replay on screen), the only time Select
/// switches cameras.
fn live(g: &Game, d: &super::cutaway::Director) -> bool {
    matches!(g.phase, Phase::Serve | Phase::Rally) && !d.showing() && !g.instant.replaying
}

/// Latches the one human's Select (their keyboard or pad) for the next tick.
fn press(
    alt: Option<ResMut<AltCamera>>,
    g: Option<Res<Game>>,
    pads: Option<Res<Pads>>,
    np: Option<Res<super::netplay::Netplay>>,
    keys: Res<ButtonInput<KeyCode>>,
    bind: Option<Res<controls::Bindings>>,
    held: Option<Res<controls::PadSlots>>,
    gamepads: Query<&Gamepad>,
) {
    let (Some(mut alt), Some(g), Some(pads), Some(bind), Some(held)) = (alt, g, pads, bind, held) else { return };
    let Some(i) = human(&g, &pads, np.is_some()) else { return };
    let Some(slot) = pads.slot_of(i, g.players.len()) else { return };
    let key = held.keys() == Some(slot) && bind.key_just_pressed(&keys, controls::Action::Camera);
    let pad = held.0[slot].and_then(|e| gamepads.get(e).ok()).is_some_and(|p| bind.pad_just_pressed(p, controls::Action::Camera));
    alt.press |= key || pad;
}

fn step(
    alt: Option<ResMut<AltCamera>>,
    g: Option<Res<Game>>,
    pads: Option<Res<Pads>>,
    np: Option<Res<super::netplay::Netplay>>,
    d: Option<Res<super::cutaway::Director>>,
    own: Option<ResMut<OwnCam>>,
    args: Res<Args>,
    mut commands: Commands,
) {
    let (Some(mut alt), Some(g), Some(pads), Some(d)) = (alt, g, pads, d) else { return };
    let alt = &mut *alt;
    let press = std::mem::take(&mut alt.press);
    let Some(i) = human(&g, &pads, np.is_some()) else { return };
    let changing_ends = matches!(g.phase, Phase::ChangeEnds(_));
    if press && alt.glide.is_none() && live(&g, &d) {
        alt.on = !alt.on;
        save(&args.iso, alt.on);
        info!("alternate camera {}", if alt.on { "on" } else { "off" });
        let (prev, now) = own_camera::shown(&g, own.as_deref());
        alt.glide = Some((GLIDE, alt.last.unwrap_or(now)));
        alt.last.get_or_insert(prev);
        if alt.on {
            alt.normal = None;
        } else {
            // the match camera freshly reset, as a new point leaves it
            let mut c = g.cam;
            c.cut();
            alt.normal = Some(c);
        }
    }
    alt.rally = if g.phase == Phase::Rally { alt.rally + 1 } else { 0 };
    let placed = g.phase == Phase::Serve && !std::mem::replace(&mut alt.serving, g.phase == Phase::Serve);
    let p = &g.players[i];
    let (spot, near, ball) = ([p.pos[0], p.pos[1], p.pos[2]], p.end > 0.0, g.flight.ball.pos);
    if matches!(g.phase, Phase::Serve | Phase::Rally) {
        alt.held = ball;
    }
    let play = alt_cam::Play::new(g.shots.max(0) as u32, (g.phase == Phase::Rally).then_some(alt.rally));
    // this tick's target: the shot behind the human, the reset match camera, else none (the shared camera shows)
    // B621: a post-point cut-away shows on the shared camera as with the normal one; the next point cuts back to a
    // fresh shot behind the human
    let target = if g.instant.replaying || d.on_screen() {
        alt.glide = None;
        alt.follow = None;
        None
    } else if alt.on {
        // a new shot for a new human or end (the change of ends), cut straight to it
        if alt.follow.as_ref().is_none_or(|f| (f.0, f.1) != (i, near)) {
            alt.follow = Some((i, near, Follow::new(&d.shots, g.players.len() == 4)));
        }
        Some(alt.follow.as_mut().expect("made").2.aim(spot, near, alt.held, play))
    } else {
        alt.follow = None;
        // the shared camera's cut to the next point (or the change of ends' glide) drops the reset one
        if placed || changing_ends {
            alt.normal = None;
        }
        alt.normal.as_mut().map(|c| {
            let players: Vec<_> = g.players.iter().map(|p| p.pos).collect();
            c.step(&Scene { players: &players, ball });
            c.view
        })
    };
    let shown = target.map(|to| match &mut alt.glide {
        Some((left, from)) => {
            *left -= 1;
            let v = swing(from, &to, alt_cam::glide_weight(*left), spot);
            if *left == 0 {
                alt.glide = None;
            }
            v
        }
        None => to,
    })
    // B628: the picture eases after the shot's view, the glide into it included
    .map(|v| match &mut alt.follow {
        Some((.., f)) => f.show(v),
        None => v,
    });
    // the fade, behind the human only
    let post = g.phase == Phase::Post;
    let blocked = alt.on
        && shown.is_some()
        && (post || g.phase == Phase::Rally && alt.rally > SETTLE)
        && g.last_hitter != i as i32
        && alt_cam::blocks(&shown.expect("checked"), spot, ball);
    alt.fade.step(blocked, post || !alt.on);
    let views = shown.map(|now| (alt.last.unwrap_or(now), now));
    alt.last = shown;
    match own {
        Some(mut o) => {
            o.alt = views;
            // the human's carrier goes once nothing is shown, the plain shared camera back
            if views.is_none() && alt.fade.alpha() == 1.0 && o.is_passive() {
                commands.remove_resource::<OwnCam>();
            }
        }
        None if views.is_some() => {
            let mut o = own_camera::passive(&g, i);
            o.alt = views;
            commands.insert_resource(o);
        }
        None => {}
    }
}

/// Draws the human see-through by the fade's alpha: every material under their figure (body, face, racket) takes
/// it as the GS alpha, blended; at 1 they get back what they had.
fn fade(
    alt: Option<Res<AltCamera>>,
    g: Option<Res<Game>>,
    pads: Option<Res<Pads>>,
    np: Option<Res<super::netplay::Netplay>>,
    figures: Query<(Entity, &Figure)>,
    children: Query<&Children>,
    mats: Query<&MeshMaterial3d<crate::gs::GsMaterial>>,
    mut materials: ResMut<Assets<crate::gs::GsMaterial>>,
    mut kept: Local<Vec<(Handle<crate::gs::GsMaterial>, f32, crate::gs::GsKey)>>,
) {
    let a = alt.as_ref().map_or(1.0, |a| a.alpha());
    if a == 1.0 {
        for (h, alpha, key) in kept.drain(..) {
            if let Some(mut m) = materials.get_mut(&h) {
                (m.uniform.color.w, m.key) = (alpha, key);
            }
        }
        return;
    }
    let (Some(g), Some(pads)) = (g, pads) else { return };
    let Some(i) = human(&g, &pads, np.is_some()) else { return };
    if kept.is_empty() {
        let Some((e, _)) = figures.iter().find(|(_, f)| f.0 == i) else { return };
        *kept = children
            .iter_descendants(e)
            .filter_map(|d| mats.get(d).ok())
            .filter_map(|m| materials.get(&m.0).map(|x| (m.0.clone(), x.uniform.color.w, x.key)))
            .collect();
    }
    for (h, alpha, key) in kept.iter() {
        let key = crate::gs::GsKey { modulate: true, blend: key.blend.or(Some(hst_data::mtl::Blend::Normal)), ..*key };
        if materials.get(h).is_some_and(|m| m.uniform.color.w != alpha * a || m.key != key) {
            let mut m = materials.get_mut(h).expect("checked");
            (m.uniform.color.w, m.key) = (alpha * a, key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::AltCamera;
    use crate::{Args, GameSpace};
    use bevy::mesh::skinning::SkinnedMeshInverseBindposes;

    /// A singles match, player 0 human (standing idle), player 1 a bot; with the alternate camera's plugin or not.
    fn app(alt: bool) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { slots: default(), connected: 0, seats: Some([true, false, false, false]), autoplay: false });
        app.init_resource::<tick_input::TickPads>().init_resource::<ButtonInput<KeyCode>>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, change_ends::plugin, own_camera::plugin));
        if alt {
            app.add_plugins(super::plugin);
        }
        app.add_systems(Startup, setup);
        let time = |mut g: ResMut<Game>| g.match_stats.tick();
        app.add_systems(FixedUpdate, (remember, control, simulate, time, motions, character::tick, played_out, held_ball).chain());
        app.update();
        app
    }

    /// M7: Select glides to the view behind the one human over 15 ticks, which then follows them, and back; the match
    /// underneath (the shared camera the cut-aways and voices read included) plays exactly as without it (skipped
    /// without the ISO).
    #[test]
    fn select_glides_behind_the_human_and_leaves_the_match_alone() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let (mut a, mut b) = (app(true), app(false));
        a.insert_resource(AltCamera { press: true, ..default() });
        let state = |app: &App| {
            let g = app.world().resource::<Game>();
            (g.cam.view, g.prev_view, g.flight.ball.pos, g.players.iter().map(|p| p.pos).collect::<Vec<_>>(), g.score.clone())
        };
        let mut glides = Vec::new();
        for tick in 0..600 {
            if tick == 300 {
                a.world_mut().resource_mut::<AltCamera>().press = true;
            }
            a.world_mut().run_schedule(FixedUpdate);
            b.world_mut().run_schedule(FixedUpdate);
            assert!(state(&a) == state(&b), "tick {tick}: the match changed");
            let w = a.world();
            let alt = w.resource::<AltCamera>();
            glides.push(alt.glide.map(|g| g.0));
            if tick == 299 {
                assert!(alt.on && alt.glide.is_none());
                let (g, own) = (w.resource::<Game>(), w.get_resource::<own_camera::OwnCam>());
                let (_, now) = own_camera::shown(g, own);
                assert_ne!(now, g.cam.view, "the shared camera shows");
                let z = g.players[0].pos[2];
                assert!(now.eye[2] * z > 0.0 && now.eye[2].abs() > z.abs() + 5.0, "eye {:?} not behind the human at z {z}", now.eye);
                assert!((now.fov - 0.3054).abs() < 1e-3, "fov {}", now.fov);
            }
        }
        let (on, off) = (&glides[..16], &glides[300..316]);
        let want: Vec<_> = (1..15).rev().map(Some).chain([None, None]).collect();
        assert_eq!(on, want, "glide on");
        assert_eq!(off, want, "glide off");
        let alt = a.world().resource::<AltCamera>();
        assert!(!alt.on && alt.follow.is_none());
    }

    /// B622: Select does nothing while the players can't move (the point over, the change of ends, a replay), and
    /// switches again once the serve is up (skipped without the ISO).
    #[test]
    fn select_is_ignored_while_play_is_stopped() {
        use bevy::ecs::system::RunSystemOnce;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let mut a = app(true);
        for set in [|g: &mut Game| g.phase = Phase::Post, |g: &mut Game| g.phase = Phase::ChangeEnds(0), |g: &mut Game| g.instant.replaying = true] {
            let w = a.world_mut();
            let mut g = w.resource::<Game>().clone();
            set(&mut g);
            let mut alt = w.resource_mut::<AltCamera>();
            alt.press = true;
            let on = alt.on;
            w.insert_resource(g);
            w.run_system_once(super::step).expect("step");
            assert_eq!(w.resource::<AltCamera>().on, on, "switched while stopped");
            assert!(!w.resource::<AltCamera>().press, "the press stayed latched");
        }
        let w = a.world_mut();
        let mut g = w.resource::<Game>().clone();
        (g.phase, g.instant.replaying) = (Phase::Serve, false);
        let on = w.resource::<AltCamera>().on;
        w.resource_mut::<AltCamera>().press = true;
        w.insert_resource(g);
            w.run_system_once(super::step).expect("step");
        assert_ne!(w.resource::<AltCamera>().on, on, "no switch in the serve");
    }

    /// B621: with the alternate camera on, a post-point cut-away takes the picture as with the normal camera (from the
    /// shot's first step), and the next point cuts back behind the human (skipped without the ISO).
    #[test]
    fn cut_aways_show_in_the_alternate_camera() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        // the match all bots (an idle human never serves); the camera, stepped after each tick, follows player 0
        let mut a = app(false);
        a.insert_resource(AltCamera { on: true, ..default() });
        let pads = |autoplay| Pads { slots: default(), connected: 0, seats: Some([true, false, false, false]), autoplay };
        let (mut shots, mut was, mut back) = (0, false, 0);
        for _ in 0..60 * 120 {
            a.insert_resource(pads(true));
            a.world_mut().run_schedule(FixedUpdate);
            a.insert_resource(pads(false));
            bevy::ecs::system::RunSystemOnce::run_system_once(a.world_mut(), super::step).unwrap();
            let w = a.world();
            let (g, own) = (w.resource::<Game>(), w.get_resource::<own_camera::OwnCam>());
            let on = w.resource::<cutaway::Director>().on_screen();
            let (prev, now) = own_camera::shown(g, own);
            if on {
                assert_eq!((prev, now), (g.prev_view, g.cam.view), "cut-away {shots}: the picture isn't the shot");
            } else if was {
                // the cut back: no blend from the shot
                let z = g.players[0].pos[2];
                assert_eq!(prev, now, "the cut back blends");
                assert!(now.eye[2] * z > 0.0 && now.eye[2].abs() > z.abs() + 5.0, "eye {:?} not behind the human at z {z}", now.eye);
                back += 1;
            }
            shots += (on && !was) as u32;
            was = on;
            if back == 2 {
                break;
            }
        }
        assert_eq!(back, 2, "{shots} cut-aways, {back} back behind the human");
    }
}

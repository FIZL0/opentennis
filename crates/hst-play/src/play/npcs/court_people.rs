//! M8f7: a court mod's people (`npcs.json`, the mods repo's standard §2b): its gallery, ball kids, line judges and
//! mascots stand where the mod puts them, each looping its `idle` clip and switching to a cue's clip as the match
//! goes. A reaction comes in the tick the disc gallery's does (`Game::applause`, left by `simulate` for
//! [`super::step`]): a game or set won claps, the match's end is `win` / `lose` for the side the gallery favours
//! (`Game::jingle` 3 / 4), an error groans, any other point cheers; a line judge points at an out call. Each person
//! starts a few ticks after its neighbour and goes back to its idle about two seconds later. Between points the
//! gallery is `serve` while a point is set up, `rally` once a rally is long, `change_ends` at a change of ends.
//! A cue's sound (one of the mod's files, picked by the tick) plays once from the people's middle, on top of the
//! disc gallery's applause.
//!
//! The people read the match and never write it: their own state is kept outside `Game`, their random picks hash
//! the tick (never the court's generator), and a re-run tick (rollback) only re-shows the same cue.

use std::sync::Arc;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use hst_sim::sound;

use super::super::{heard, Game, Phase};
use crate::audio::{Sound, SoundBank};
use crate::court_mod::{Kept, Looped, Spawned, Store};
use crate::gs::GsMaterial;
use crate::mods_court::{MatchCourt, Role};

/// Ticks a reaction lasts.
const REACT: u32 = 120;
/// Shots in a rally from which the gallery watches it (`rally`).
const LONG_RALLY: i32 = 8;
/// A person starts its reaction up to this many ticks after the cue.
const STAGGER: u32 = 16;
/// How far a line judge turns its head toward the ball (radians either way).
const HEAD_TURN: f32 = 1.2;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Crowd>()
        .add_systems(PostStartup, (spawn.after(super::setup), halves.after(spawn)).run_if(resource_exists::<MatchCourt>))
        .add_systems(FixedUpdate, cue.after(crate::character::tick).before(super::step).run_if(resource_exists::<MatchCourt>))
        .add_systems(Update, show.after(super::super::camera).after(super::super::split_view::split).run_if(resource_exists::<MatchCourt>));
}

/// One of the mod's people: its place in `npcs.json`, how it follows the match, its clips and the one showing (and
/// since when, s), and a line judge's head (its rest turn once seen).
#[derive(Component)]
struct Someone {
    index: usize,
    role: Role,
    facing: Vec2,
    billboard: bool,
    clips: Vec<(String, Looped)>,
    playing: Option<(usize, f32)>,
    head: Option<(Entity, Option<Quat>)>,
    /// A card's copy for the split view's second half.
    twin: Option<Twin>,
}

/// B639: a card's second instance, drawn by the split view's second half only (`twin_view::BOTTOM`) and turned to
/// its camera, playing its own clips (the back ones by that camera) on its own nodes; hidden while there's no split.
struct Twin {
    root: Entity,
    clips: Vec<(String, Looped)>,
    playing: Option<(usize, f32)>,
    head: Option<(Entity, Option<Quat>)>,
}

/// The people's reactions (per person: cue, first tick, tick it ends), the phase last tick, and the cue sounds.
#[derive(Resource, Default)]
struct Crowd {
    reacting: Vec<Option<(&'static str, u32, u32)>>,
    post: bool,
    sounds: Option<(Arc<SoundBank>, Vec<String>)>,
    middle: [f32; 3],
}

/// What a tick tells the people.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct News {
    /// The disc gallery's reaction to the point decided this tick.
    applause: Option<sound::Reaction>,
    /// The jingle it brings: 1 a game, 2 a set, 3 / 4 the match won / lost by the side shown.
    jingle: Option<u8>,
    /// The point (or serve) ended on an out call this tick.
    out: bool,
}

/// The reaction `role` shows to `news`, if any.
fn reaction(role: Role, news: &News) -> Option<&'static str> {
    match role {
        Role::Gallery | Role::Mascot => {
            let r = news.applause?;
            Some(match news.jingle {
                Some(3) => "win",
                Some(4) => "lose",
                _ if r.event == Some(1) => "groan",
                Some(1 | 2) => "clap",
                _ => "cheer",
            })
        }
        Role::Linejudge => news.out.then_some("point"),
        Role::Ballkid => None,
    }
}

/// What `role` shows between reactions.
fn base(role: Role, phase: &Phase, shots: i32) -> &'static str {
    match (role, phase) {
        (_, Phase::ChangeEnds(_)) => "change_ends",
        (Role::Gallery | Role::Mascot | Role::Ballkid, Phase::Serve) => "serve",
        (Role::Gallery | Role::Mascot, Phase::Rally) if shots >= LONG_RALLY => "rally",
        _ => "idle",
    }
}

/// The cue a model lacking `cue` plays first (then `idle`).
fn stand_in(cue: &str) -> Option<&'static str> {
    match cue {
        "win" => Some("cheer"),
        "lose" => Some("groan"),
        _ => None,
    }
}

/// The clip for `cue` among `names`, seen from behind or not: `<cue>_back`, then `idle_back`, then the cue itself;
/// a missing cue its stand-in, then `idle`.
fn clip(names: &[&str], cue: &str, back: bool) -> Option<usize> {
    let at = |n: &str| names.iter().position(|k| *k == n);
    let alt = stand_in(cue);
    let behind = || at(&format!("{cue}_back")).or_else(|| alt.and_then(|a| at(&format!("{a}_back")))).or_else(|| at("idle_back"));
    back.then(behind).flatten().or_else(|| at(cue)).or_else(|| alt.and_then(at)).or_else(|| at("idle"))
}

/// Ticks person `i` waits before reacting.
fn stagger(i: usize) -> u32 {
    ((i as u32).wrapping_mul(0x9e37_79b1) >> 16) % STAGGER
}

/// The camera is behind a person at `pos` facing `facing` (game x, z).
fn behind(eye: Vec3, pos: Vec3, facing: Vec2) -> bool {
    (eye.x - pos.x) * facing.x + (eye.z - pos.z) * facing.y < 0.0
}

#[allow(clippy::too_many_arguments)]
fn spawn(
    mut commands: Commands,
    court: Res<MatchCourt>,
    args: Res<crate::Args>,
    root: Query<Entity, With<crate::GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GsMaterial>>,
    mut binds: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut kept: ResMut<Kept>,
    mut crowd: ResMut<Crowd>,
) {
    let (Some(people), Ok(root)) = (&court.0.people, root.single()) else { return };
    let c = &court.0;
    let mut store = Store { meshes: &mut meshes, images: &mut images, materials: &mut materials, binds: &mut binds, kept: &mut kept, upscaled: None };
    let files: Vec<_> = people.models.iter().map(|(name, path, _)| crate::mods::open(path).map_err(|e| warn!("court mod {}: model `{name}`: {e}", c.id)).ok()).collect();
    let mut draws = 0;
    for (i, p) in people.people.iter().enumerate() {
        let Some((doc, blob)) = &files[p.model] else { continue };
        let billboard = people.models[p.model].2;
        let turn = Quat::from_rotation_y(p.facing.x.atan2(p.facing.y));
        let e = commands.spawn((Transform::from_translation(p.pos).with_rotation(turn), Visibility::default(), ChildOf(root))).id();
        let mut sp = Spawned::default();
        store.upscaled = crate::mods::upscaled_in(&c.dir, &people.models[p.model].1);
        match crate::court_mod::scene(&mut commands, &mut store, doc, blob, e, args.singles, c.light.as_ref(), false, Some(&mut sp)) {
            Ok(n) => draws += n,
            Err(err) => warn!("court mod {}: person {i}: {err}", c.id),
        }
        let head_of = |sp: &Spawned| (p.role == Role::Linejudge).then(|| sp.nodes.iter().find(|n| n.0.eq_ignore_ascii_case("head")).map(|n| (n.1, None))).flatten();
        let head = head_of(&sp);
        let twin = billboard.then(|| {
            let root = commands.spawn((Transform::from_translation(p.pos).with_rotation(turn), Visibility::Hidden, ChildOf(root))).id();
            let mut sp = Spawned::default();
            if let Err(err) = crate::court_mod::scene(&mut commands, &mut store, doc, blob, root, args.singles, c.light.as_ref(), false, Some(&mut sp)) {
                warn!("court mod {}: person {i}'s second view copy: {err}", c.id);
            }
            Twin { root, head: head_of(&sp), clips: sp.clips, playing: None }
        });
        commands.entity(e).insert(Someone { index: i, role: p.role, facing: p.facing, billboard, clips: sp.clips, playing: None, head, twin });
    }
    info!("court mod {}: {} people, {draws} draws", c.id, people.people.len());
    crowd.reacting = vec![None; people.people.len()];
    let n = people.people.len().max(1) as f32;
    crowd.middle = (people.people.iter().map(|p| p.pos).sum::<Vec3>() / n).to_array();
    // the cue sounds: one program per cue, a key per file
    let mut wav = std::collections::HashMap::new();
    let mut cues = Vec::new();
    for (cue, files) in &people.sounds {
        let takes: Vec<(Arc<[i16]>, u32)> = files.iter().filter_map(|f| crate::modfs::read(f).ok().and_then(|d| crate::audio::pcm(&d)).map(|(s, r)| (s.into(), r)).or_else(|| { warn!("{}: not a PCM16 wav, skipped", f.display()); None })).collect();
        if !takes.is_empty() {
            wav.insert(cues.len(), takes);
            cues.push(cue.clone());
        }
    }
    if !cues.is_empty() {
        crowd.sounds = Some((Arc::new(SoundBank { hd: Vec::new(), bd: Vec::new(), wav, takes: Default::default(), stream: None }), cues));
    }
}

/// The tick's cues: who reacts to what, and the cue's sound.
fn cue(g: Res<Game>, heard: Res<heard::Heard>, mut played: ResMut<heard::Played>, mut crowd: ResMut<Crowd>, people: Query<&Someone>, audio: Option<Res<Sound>>) {
    let t = heard.tick;
    let post = g.phase == Phase::Post;
    let entered = post && !crowd.post;
    crowd.post = post;
    let news = News { applause: g.applause, jingle: g.jingle, out: entered && matches!(g.flight.call, hst_sim::judge::Call::Out | hst_sim::judge::Call::NetOut) };
    if news == News::default() {
        return;
    }
    let mut heard_cues: Vec<&'static str> = Vec::new();
    for p in &people {
        let Some(cue) = reaction(p.role, &news) else { continue };
        let from = t + stagger(p.index);
        let until = if matches!(cue, "win" | "lose") { u32::MAX } else { from + REACT };
        if let Some(r) = crowd.reacting.get_mut(p.index) {
            *r = Some((cue, from, until));
        }
        if !heard_cues.contains(&cue) {
            heard_cues.push(cue);
        }
    }
    let (Some(audio), Some((bank, cues))) = (audio, &crowd.sounds) else { return };
    for cue in heard_cues {
        let Some(program) = cues.iter().position(|c| c == cue).or_else(|| stand_in(cue).and_then(|a| cues.iter().position(|c| c == a))) else { continue };
        if !played.first(t, ("court people", cue)) {
            continue;
        }
        let n = bank.wav[&program].len() as u32;
        let key = (t.wrapping_mul(0x9e37_79b1) >> 8) % n;
        let volume = if matches!(cue, "win" | "lose") { 0x66 } else { 0x4c };
        audio.play_at(bank, sound::Play { slot: 6, program: program as u8, key: key as u8, volume, speed: 1.0 }, crowd.middle);
    }
}

/// Each person's clip at the frame's time, turned to the camera if a card, a line judge's head after the ball.
#[allow(clippy::type_complexity)]
fn show(
    time: Res<Time>,
    g: Res<Game>,
    heard: Res<heard::Heard>,
    crowd: Res<Crowd>,
    cam: Query<&Transform, (With<crate::Orbit>, Without<Someone>)>,
    mut people: Query<(&mut Someone, &mut Transform), Without<crate::Orbit>>,
    mut nodes: Query<&mut Transform, (Without<Someone>, Without<crate::Orbit>)>,
    mut forced: Local<Option<Option<String>>>,
    mut shown: Query<&mut Visibility>,
    second: crate::twin_view::Second,
) {
    // `HST_PEOPLE_CUE=<cue>`: everyone shows that cue (for `--shot`s)
    let forced = forced.get_or_insert_with(|| std::env::var("HST_PEOPLE_CUE").ok()).as_deref();
    let now = time.elapsed_secs();
    let t = heard.tick;
    // game space is Bevy's turned a half-turn about X
    let eye = cam.single().map(|c| Vec3::new(c.translation.x, -c.translation.y, -c.translation.z)).ok();
    let ball = Vec3::from(g.flight.ball.pos);
    for (mut p, mut tf) in &mut people {
        let p = &mut *p;
        let cue = match crowd.reacting.get(p.index).copied().flatten() {
            _ if forced.is_some() => forced.unwrap_or_default(),
            Some((cue, from, until)) if (from..until).contains(&t) => cue,
            _ => base(p.role, &g.phase, g.shots),
        };
        if let (true, Some(eye)) = (p.billboard, eye) {
            let d = eye - tf.translation;
            tf.rotation = Quat::from_rotation_y(d.x.atan2(d.z));
        }
        let at = tf.translation;
        act(&p.clips, &mut p.playing, &mut p.head, cue, eye, at, p.facing, ball, now, &mut nodes);
        let Some(tw) = &mut p.twin else { continue };
        // the split view's second half: the card turned to its camera, or hidden while there's none
        let eye = second.eye().map(|c| Vec3::new(c.translation.x, -c.translation.y, -c.translation.z));
        if let Ok(mut v) = shown.get_mut(tw.root) {
            v.set_if_neq(if eye.is_some() { Visibility::Inherited } else { Visibility::Hidden });
        }
        let (Some(eye), Ok(mut root)) = (eye, nodes.get_mut(tw.root)) else { continue };
        let d = eye - at;
        *root = Transform::from_translation(at).with_rotation(Quat::from_rotation_y(d.x.atan2(d.z)));
        act(&tw.clips, &mut tw.playing, &mut tw.head, cue, Some(eye), at, p.facing, ball, now, &mut nodes);
    }
}

/// One person's `cue` clip at `now` (its back clip if the camera at `eye` is behind it) and a line judge's head after
/// the ball, for someone standing at `at` facing `facing`.
#[allow(clippy::too_many_arguments)]
fn act(
    clips: &[(String, Looped)],
    playing: &mut Option<(usize, f32)>,
    head: &mut Option<(Entity, Option<Quat>)>,
    cue: &str,
    eye: Option<Vec3>,
    at: Vec3,
    facing: Vec2,
    ball: Vec3,
    now: f32,
    nodes: &mut Query<&mut Transform, (Without<Someone>, Without<crate::Orbit>)>,
) {
    let back = eye.is_some_and(|eye| behind(eye, at, facing));
    let names: Vec<&str> = clips.iter().map(|c| c.0.as_str()).collect();
    if let Some(k) = clip(&names, cue, back) {
        if playing.is_none_or(|(at, _)| at != k) {
            *playing = Some((k, now));
        }
        let (_, since) = playing.unwrap();
        let l = &clips[k].1;
        if let Some((head, Some(rest))) = *head
            && let Ok(mut h) = nodes.get_mut(head)
        {
            h.rotation = rest;
        }
        l.pose((now - since) % l.length, nodes);
    }
    if let Some((head, rest)) = head {
        let Ok(mut h) = nodes.get_mut(*head) else { return };
        let rest = *rest.get_or_insert(h.rotation);
        let to = ball - at;
        let mut turn = to.x.atan2(to.z) - facing.x.atan2(facing.y);
        turn = (turn + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        h.rotation = Quat::from_rotation_y(turn.clamp(-HEAD_TURN, HEAD_TURN)) * rest;
    }
}

/// While the split view is on, the cards' own meshes are drawn by the top half only and their copies' by the bottom
/// half only (`twin_view`).
fn halves(mut commands: Commands, people: Query<(Entity, &Someone)>, children: Query<&Children>, meshes: Query<(), With<Mesh3d>>) {
    for (e, p) in &people {
        let Some(tw) = &p.twin else { continue };
        for m in children.iter_descendants(e).filter(|m| meshes.contains(*m)) {
            commands.entity(m).insert(crate::twin_view::TopHalf);
        }
        for m in children.iter_descendants(tw.root).filter(|m| meshes.contains(*m)) {
            commands.entity(m).insert(bevy::camera::visibility::RenderLayers::layer(crate::twin_view::BOTTOM));
        }
    }
}

/// For the court mod match test: the people placed, how many have reacted so far, and whether there are cue sounds.
#[cfg(test)]
pub(in crate::play) fn seen(world: &mut World) -> (usize, usize, bool) {
    let placed = world.query::<&Someone>().iter(world).count();
    let crowd = world.resource::<Crowd>();
    (placed, crowd.reacting.iter().filter(|r| r.is_some()).count(), crowd.sounds.is_some())
}

/// For the court mod match test: the cards, and those whose meshes are all the top half's with a copy all the
/// bottom half's (`twin_view`), hidden while there's no split.
#[cfg(test)]
pub(in crate::play) fn split(world: &mut World) -> (usize, usize) {
    use bevy::camera::visibility::RenderLayers;
    let bottom = RenderLayers::layer(crate::twin_view::BOTTOM);
    let mut q = world.query::<(Entity, &Someone)>();
    let cards: Vec<_> = q.iter(world).filter(|(_, p)| p.billboard).map(|(e, p)| (e, p.twin.as_ref().map(|t| t.root))).collect();
    let meshes = |world: &World, e: Entity| -> Vec<Entity> {
        let mut out = vec![];
        let mut stack = vec![e];
        while let Some(e) = stack.pop() {
            if world.get::<Mesh3d>(e).is_some() {
                out.push(e);
            }
            stack.extend(world.get::<Children>(e).into_iter().flat_map(|c| c.iter()));
        }
        out
    };
    let ok = cards
        .iter()
        .filter(|(e, twin)| {
            let Some(twin) = *twin else { return false };
            let (own, copy) = (meshes(world, *e), meshes(world, twin));
            !own.is_empty()
                && own.len() == copy.len()
                && own.iter().all(|m| world.get::<crate::twin_view::TopHalf>(*m).is_some())
                && copy.iter().all(|m| world.get::<RenderLayers>(*m) == Some(&bottom))
                && world.get::<Visibility>(twin) == Some(&Visibility::Hidden)
        })
        .count();
    (cards.len(), ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cues as the standard spells them: per role, per point; between points; the clip a model lacking one
    /// plays; the back clips; the stagger stays under its bound and differs between neighbours.
    #[test]
    fn people_cue_like_the_standard() {
        let r = |cheer, event| Some(sound::Reaction { cheer, event, chain: false });
        let news = |applause, jingle, out| News { applause, jingle, out };
        for role in [Role::Gallery, Role::Mascot] {
            assert_eq!(reaction(role, &news(r(true, Some(0)), None, false)), Some("cheer"));
            assert_eq!(reaction(role, &news(r(true, None), None, false)), Some("cheer"));
            assert_eq!(reaction(role, &news(r(false, Some(1)), None, true)), Some("groan"));
            assert_eq!(reaction(role, &news(r(true, None), Some(1), false)), Some("clap"));
            assert_eq!(reaction(role, &news(r(true, Some(1)), Some(2), false)), Some("groan"));
            assert_eq!(reaction(role, &news(r(true, None), Some(3), false)), Some("win"));
            assert_eq!(reaction(role, &news(r(true, Some(1)), Some(4), false)), Some("lose"));
            assert_eq!(reaction(role, &news(None, None, true)), None, "a fault isn't the gallery's");
        }
        assert_eq!(reaction(Role::Linejudge, &news(None, None, true)), Some("point"));
        assert_eq!(reaction(Role::Linejudge, &news(r(true, None), Some(1), false)), None);
        assert_eq!(reaction(Role::Ballkid, &news(r(true, None), Some(3), true)), None);
        assert_eq!(base(Role::Gallery, &Phase::Serve, 0), "serve");
        assert_eq!(base(Role::Gallery, &Phase::Rally, LONG_RALLY - 1), "idle");
        assert_eq!(base(Role::Mascot, &Phase::Rally, LONG_RALLY), "rally");
        assert_eq!(base(Role::Ballkid, &Phase::Rally, 20), "idle");
        assert_eq!(base(Role::Ballkid, &Phase::Serve, 0), "serve");
        assert_eq!(base(Role::Linejudge, &Phase::Serve, 0), "idle");
        assert_eq!(base(Role::Linejudge, &Phase::ChangeEnds(3), 0), "change_ends");
        assert_eq!(base(Role::Gallery, &Phase::Post, 9), "idle");
        let card = ["idle", "idle_back", "groan_back", "clap_back", "cheer_back", "groan", "clap", "cheer"];
        assert_eq!(clip(&card, "cheer", false), Some(7));
        assert_eq!(clip(&card, "cheer", true), Some(4));
        assert_eq!(clip(&card, "win", false), Some(7), "win stands in as cheer");
        assert_eq!(clip(&card, "lose", true), Some(2), "lose seen from behind: groan's back");
        assert_eq!(clip(&card, "serve", false), Some(0));
        assert_eq!(clip(&card, "serve", true), Some(1), "a cue without its back: idle's");
        let stands = ["idle", "groan", "clap", "cheer"];
        assert_eq!(clip(&stands, "clap", true), Some(2), "no back clips: the cue itself");
        assert_eq!(clip(&["wave"], "cheer", false), None, "no idle, nothing to play");
        assert!((0..64).all(|i| stagger(i) < STAGGER));
        assert!((0..63).filter(|&i| stagger(i) != stagger(i + 1)).count() > 40);
        assert!(behind(Vec3::new(0.0, -2.0, 30.0), Vec3::new(0.0, 0.0, 20.0), Vec2::new(0.0, -1.0)));
        assert!(!behind(Vec3::new(0.0, -2.0, 0.0), Vec3::new(0.0, 0.0, 20.0), Vec2::new(0.0, -1.0)));
    }
}

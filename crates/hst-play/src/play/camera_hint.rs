//! B651 (ours, by the user's request): on a tall (portrait) window the match keeps the normal camera, never switching
//! by itself, and a hint over the picture says Select switches to the singles camera (the alternate one low behind
//! the player, `alt_camera.rs`) wherever that switch works (one human on this machine, offline) in live play. Once
//! the player has switched to it, anywhere and any shape of window, the hint never shows again: `camera_hint.txt`
//! in the per-user data folder (the browser's localStorage) remembers it. Presentation only: nothing here reaches
//! the sim.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::alt_camera::{self, AltCamera};
use super::controls::{Action, Bindings};
use super::legend::{self, Act, Item, Last, Style};
use super::{Game, Pads, Phase};
use crate::Args;

/// The hint's place on the 640×448 screen: centred, under the score panel's top rows.
const RECT: [f32; 4] = [170.0, 72.0, 300.0, 18.0];

pub fn plugin(app: &mut App) {
    app.init_resource::<Hint>().add_systems(Startup, (load, setup)).add_systems(PostUpdate, (judge, draw).chain().before(bevy::ui::UiSystems::Layout));
}

#[derive(Resource, Default)]
struct Hint {
    /// The player has switched to the singles camera once.
    seen: bool,
    /// It shows this frame.
    want: bool,
}

/// The hint's root node.
#[derive(Component)]
struct Root;

/// `camera_hint.txt` in the per-user data folder.
fn path(iso: &str) -> std::path::PathBuf {
    crate::store::path(iso, "camera_hint.txt")
}

fn load(mut hint: ResMut<Hint>, args: Option<Res<Args>>) {
    hint.seen = args.is_some_and(|a| crate::store::read_to_string(path(&a.iso)).is_ok_and(|t| t.trim() == "seen"));
}

fn setup(mut commands: Commands) {
    let node = Node { position_type: PositionType::Absolute, justify_content: JustifyContent::Center, align_items: AlignItems::Center, border_radius: BorderRadius::all(Val::Percent(50.0)), ..default() };
    commands.spawn((Root, node, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)), GlobalZIndex(29), Visibility::Hidden));
}

/// Whether the hint shows: a tall picture, the switch works and isn't used yet, the players can move.
fn wanted(seen: bool, tall: bool, switch: bool, live: bool) -> bool {
    !seen && tall && switch && live
}

/// Marks the hint seen once the singles camera is on (saved the first time) and decides this frame's.
fn judge(w: &mut World) {
    let chosen = w.get_resource::<AltCamera>().is_some_and(|a| a.chosen());
    if chosen && !w.resource::<Hint>().seen {
        w.resource_mut::<Hint>().seen = true;
        // tests never write beside the disc
        if !cfg!(test)
            && let Some(a) = w.get_resource::<Args>()
            && let Err(e) = crate::store::write(path(&a.iso), "seen\n")
        {
            warn!("camera_hint.txt: {e}");
        }
    }
    let tall = w.query_filtered::<&Window, With<PrimaryWindow>>().single(w).is_ok_and(|win| crate::play::widescreen::scene_aspect(win) < 1.0);
    let switch = match (w.get_resource::<Game>(), w.get_resource::<Pads>()) {
        (Some(g), Some(p)) => alt_camera::human(g, p, w.contains_resource::<super::netplay::Netplay>()).is_some(),
        _ => false,
    };
    let live = w.get_resource::<Game>().is_some_and(|g| matches!(g.phase, Phase::Serve | Phase::Rally) && !g.instant.replaying)
        && !w.get_resource::<super::cutaway::Director>().is_some_and(|d| d.showing())
        && !super::front_end::paused(w)
        && !super::intro::running(w);
    let want = wanted(w.resource::<Hint>().seen, tall, switch, live);
    w.resource_mut::<Hint>().want = want;
}

/// The hint's chip for the device last used: the touch pad's SELECT, the keyboard's camera key, a pad's Select.
fn item(last: Last, bind: &Bindings) -> (Style, Item) {
    let keys = |k: Vec<String>| Item { act: Act::Hand, label: "Singles camera".into(), keys: k, dim: [false; 2] };
    match last {
        Last::Touch => (Style::Keys, keys(vec!["SELECT".into()])),
        Last::Keys => (Style::Keys, keys(bind.key(Action::Camera).into_iter().collect())),
        Last::Pad(e) => (legend::dev_style(Some(e)), keys(vec![])),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    hint: Res<Hint>,
    last: Res<Last>,
    bind: Option<Res<Bindings>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut root: Query<(Entity, &mut Node, &mut Visibility), With<Root>>,
    mut images: ResMut<Assets<Image>>,
    mut shown: Local<String>,
    mut art: Local<Option<(String, Handle<Image>, f32)>>,
) {
    let (Ok(win), Ok((e, mut node, mut vis))) = (window.single(), root.single_mut()) else { return };
    let chip = bind.filter(|_| hint.want).map(|b| item(*last, &b));
    vis.set_if_neq(if chip.is_some() { Visibility::Inherited } else { Visibility::Hidden });
    let Some((style, it)) = chip else { return };
    let k = format!("{style:?} {it:?}");
    if art.as_ref().is_none_or(|(a, ..)| *a != k) {
        let Some((img, aspect)) = legend::chip_image(style, &it) else { return };
        *art = Some((k.clone(), images.add(img), aspect));
    }
    let Some((_, img, aspect)) = art.clone() else { return };
    let aspect_w = crate::play::widescreen::aspect(win);
    let share = crate::play::widescreen::share(aspect_w);
    let drop = super::panel::touch_drop(win, aspect_w);
    let [x, y, w, h] = RECT;
    let (top, height) = super::panel::vplace_clear(y, h, aspect_w, drop);
    let (left, width) = crate::play::widescreen::place(x, w, share, crate::play::widescreen::Anchor::Centre);
    let key = format!("{k} {left} {top} {width} {height}");
    if *shown == key {
        return;
    }
    *shown = key;
    // the chip 5/6 of the bar's height, centred in it at its own aspect
    let view = crate::play::widescreen::view_size(win);
    let chip_h = height / 100.0 * view.y * 10.0 / 12.0;
    let chip_w = (chip_h * aspect).min(width / 100.0 * view.x - chip_h * 0.6);
    let bar_w = (chip_w + chip_h * 0.8) / view.x * 100.0;
    (node.left, node.width) = (Val::Percent(left + (width - bar_w) / 2.0), Val::Percent(bar_w));
    (node.top, node.height) = (Val::Percent(top), Val::Percent(height));
    commands.entity(e).despawn_related::<Children>();
    commands.entity(e).with_children(|p| {
        p.spawn((ImageNode::new(img), Node { width: Val::Px(chip_w), height: Val::Px(chip_w / aspect), flex_shrink: 0.0, ..default() }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_only_tall_live_and_unused() {
        assert!(wanted(false, true, true, true));
        assert!(!wanted(true, true, true, true), "used once: never again");
        assert!(!wanted(false, false, true, true), "wide window");
        assert!(!wanted(false, true, false, true), "no singles camera here (two humans, online)");
        assert!(!wanted(false, true, true, false), "not live play");
    }

    #[test]
    fn chip_per_device() {
        let (b, _) = Bindings::parse("");
        let (s, it) = item(Last::Touch, &b);
        assert_eq!((s, it.keys.as_slice()), (Style::Keys, ["SELECT".to_string()].as_slice()));
        let (s, it) = item(Last::Keys, &b);
        assert_eq!((s, it.keys.as_slice()), (Style::Keys, ["C".to_string()].as_slice()));
        assert!(legend::chip_image(Style::Ps, &item(Last::Pad(Entity::PLACEHOLDER), &b).1).is_some());
    }
}

//! B301 (ours, by the user's request): the match's keyboard hints. While the keyboard plays a seat, the menus' legend strip (`legend.rs`, bottom right under the 4:3 screen's description bar, in
//! keycaps) lists the match's keys from `controls.txt`: move, the three shots (which serve too), the emotes, the camera
//! switch (only where it works: humans on this machine, offline; `alt_camera::offered`) and pause. A pad pressed
//! doesn't hide it (the keyboard player still plays; user 2026-10-23), a touch does, and so does the Key Hints setting (Settings → Display,
//! `key_hints` in settings.txt, on by default). Whatever else fills the legend (the pause menu, its Settings screens,
//! the result screen) takes the strip instead. Presentation only: nothing here reaches the sim.

use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

use super::controls::{Action, Bindings, PadSlots};
use super::legend::{self, Act, Item, Last, Legend};

/// The Key Hints setting, as the main menu (and the pause menu's Settings) last had it.
pub static ON: AtomicBool = AtomicBool::new(true);

/// The match's keys, by the bindings (a key-less action drops its cap; an action with none drops out); `camera`: the
/// alternate camera's switch works in this match.
fn items(bind: &Bindings, camera: bool) -> Vec<Item> {
    let k = |a: Action| bind.key(a);
    let caps = |acts: &[Action]| acts.iter().filter_map(|&a| k(a)).collect::<Vec<_>>();
    let mut v = vec![
        (Act::Move, "Move", caps(&[Action::Up, Action::Left, Action::Down, Action::Right])),
        (Act::Key, "Topspin", caps(&[Action::Normal])),
        (Act::Key, "Slice", caps(&[Action::Cut])),
        (Act::Key, "Lob", caps(&[Action::Lob])),
        (Act::Key, "Emotes", caps(&[Action::EmoteUp, Action::EmoteDown, Action::EmoteLeft, Action::EmoteRight])),
    ];
    if camera {
        v.push((Act::Key, "Camera", caps(&[Action::Camera])));
    }
    v.push((Act::Key, "Pause", caps(&[Action::Pause])));
    v.into_iter().filter(|(_, _, keys)| !keys.is_empty()).map(|(act, label, keys)| Item { act, label: label.into(), keys, dim: [false; 2] }).collect()
}

/// Whether the hints show: the setting on, the keyboard playing a seat (no autoplay), whatever pad was pressed
/// since; not after a touch.
fn shown(on: bool, last: Last, keys_seat: bool, autoplay: bool) -> bool {
    on && last != Last::Touch && keys_seat && !autoplay
}

fn fill(
    bind: Res<Bindings>,
    last: Res<Last>,
    slots: Res<PadSlots>,
    pads: Option<Res<super::Pads>>,
    g: Option<Res<super::Game>>,
    np: Option<Res<super::netplay::Netplay>>,
    gr: Option<Res<crate::graphics::Graphics>>,
    mut out: ResMut<Legend>,
) {
    let seat = slots.keys().is_some_and(|k| pads.as_ref().is_none_or(|p| p.seats.is_none_or(|h| h[k])));
    if !out.0.is_empty() || !shown(ON.load(Ordering::Relaxed), *last, seat, pads.as_ref().is_some_and(|p| p.autoplay)) {
        return;
    }
    // B658: whatever the local humans' mix
    let original = gr.is_some_and(|gr| gr.original);
    let camera = g.zip(pads.as_ref()).is_some_and(|(g, p)| super::alt_camera::offered(&g, p, np.is_some(), original));
    out.one(Last::Keys, items(&bind, camera));
}

pub fn plugin(app: &mut App) {
    legend::plugin(app);
    app.add_systems(PostUpdate, fill.before(legend::draw));
}

#[test]
fn hints_follow_the_bindings_and_the_keyboard() {
    let (b, _) = Bindings::parse("topspin = Z\nlob = PadNorth\n");
    let got: Vec<_> = items(&b, false).into_iter().map(|i| format!("{} {}", i.label, i.keys.join(" "))).collect();
    // lob has no key left: no entry
    assert_eq!(got, ["Move W A S D", "Topspin Z", "Slice K", "Emotes 1 2 3 4", "Pause Esc"]);
    // with one human offline, the camera switch too
    let got: Vec<_> = items(&b, true).into_iter().map(|i| format!("{} {}", i.label, i.keys.join(" "))).collect();
    assert_eq!(got[4..], ["Camera C", "Pause Esc"]);
    assert!(shown(true, Last::Keys, true, false));
    assert!(!shown(false, Last::Keys, true, false), "setting off");
    assert!(shown(true, Last::Pad(Entity::PLACEHOLDER), true, false), "a pad pressed alongside the keyboard player");
    assert!(!shown(true, Last::Touch, true, false), "touch");
    assert!(!shown(true, Last::Keys, false, false), "keyboard has no seat");
    assert!(!shown(true, Last::Keys, true, true), "autoplay");
}

//! M91a1: the on-screen pad of the native iPhone app, as the browser's (`web/touch.js`, B247): a virtual gamepad the
//! game reads like any controller, so menus and matches take it through the same bindings as a real pad (✕ = shot / OK,
//! ○ = slice / back, △ = lob, □, L1/R1, Select = swap hand, Start = pause; in a match L1/R1 also emote, `emote::touch`
//! knowing it by its name). Beside Select, while online chat is on, a speech bubble opens the chat box (`chat::BUTTON`). Face buttons in a diamond for the right thumb, the shoulders and Select/Start along the top,
//! clear of the notch and the home indicator (the window's safe area). It shows while no real controller is connected:
//! one connecting takes it away in the same frame, before any seat or slot sees both (B247). B656: unless it holds a
//! seat (it pressed Start on the splash, `seats::hold_touch`): then it stays and keeps playing as 1P, and the
//! controller joins as the next player.
//!
//! In the menus, which take taps themselves (B221), the stick is a fixed circle at the bottom left, so taps anywhere else
//! reach the menu; in a match (the pause and result screens are menus) it appears wherever the left thumb lands in the
//! screen's left part. A touch keeps the control it landed on until it lifts, wherever it slides; the menus never see
//! it (its `Touches` press is taken back). Each finger counts on its own (B165, B274): a second finger on a button
//! neither presses it again nor lets it go when it lifts, and a button is down for at least one frame, so a tap shorter
//! than a frame still counts. A press gives the phone's light haptic tick (Settings → Mobile → Vibration).
//!
//! M102: the Android app's too (`tools/apk.sh`), where native touches arrive as `TouchInput` like the iPhone's; its safe
//! area is the edge-to-edge window's system bars and camera cutout (`android.rs`), read by `widescreen` (M106), which
//! fits the view into it too. B421: at the Aspect Ratio's Stretch the view takes the whole screen, notch and all; the
//! pad's buttons still keep clear of the safe area (`Layout::new`), where a thumb would hit the island or the system's
//! home gesture.
//!
//! M102: it draws on its own camera, straight on the window: the UI (`hud_gamma`) is drawn only into the Aspect
//! Ratio's centred view, 16:9 by default on a phone, which would clip a pad laid out for the whole window (the
//! touches are the window's). It hides under the loading screen; it covers the chat.
//!
//! The base app's (`mode.rs`): its pad entity and resource outlive every mode switch. On with `target_os = "ios"` or
//! `"android"`;
//! `HST_TOUCH=1` puts it on anywhere (a desktop touch screen, `--shot`s of its layout). Never touches the sim: it is
//! only a gamepad.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::input::gamepad::{GamepadConnection, GamepadConnectionEvent, GamepadInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::controls::{Action, Bindings};
use super::legend::{Act, Drawn, Item};

/// Sony's vendor id: the legend's hints show ✕ ○ □ △ for this pad on Auto (the browser's says the same in its id).
pub(super) const SONY: u16 = 0x054c;
/// The pad's name: `emote::touch` knows it by its start ("Touch controls", as the browser's).
const NAME: &str = if cfg!(target_os = "android") { "Touch controls (Android)" } else { "Touch controls (iPhone)" };
/// The Vibration setting (Settings → Mobile), as the menus last had it.
pub static BUZZ: AtomicBool = AtomicBool::new(true);

/// Whether this app is played by touch: the iPhone or Android app, or anything with `HST_TOUCH` set.
pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| cfg!(any(target_os = "ios", target_os = "android")) || std::env::var_os("HST_TOUCH").is_some_and(|v| v != "0"))
}

/// The buttons: ✕ ○ □ △ (face, the symbol with what it does under it) then L1 R1 Select Start (small, only the word),
/// each labelled by `LABELS`.
const KEYS: [GamepadButton; 8] = [
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::West,
    GamepadButton::North,
    GamepadButton::LeftTrigger,
    GamepadButton::RightTrigger,
    GamepadButton::Select,
    GamepadButton::Start,
];
const FACE: usize = 4;

/// B662: the words on the buttons (`KEYS` order): what each does on the screen as it stands, blank where it does
/// nothing, and whether a match is being played (□ is hidden then: it does nothing in a match). `relabel` sets them;
/// the browser's pad (`web/touch.js`) gets the same ones.
static LABELS: Mutex<(bool, [String; 8])> = Mutex::new((false, [const { String::new() }; 8]));
/// □'s index in `KEYS`.
const SQUARE: usize = 2;

/// B662: the buttons' words: in a match (`play`: no menu over it) the pad's bindings, L1 / R1 the emotes (B271) and
/// Select the camera where it switches one (`camera`); else what the menu's legend lists for the touch player (`items`).
fn labels(items: &[Item], play: bool, camera: bool, bind: &Bindings) -> [String; 8] {
    use GamepadButton as B;
    std::array::from_fn(|k| {
        let b = KEYS[k];
        if !play {
            return items.iter().find(|it| on_button(it.act, b)).map(|it| it.label.clone()).unwrap_or_default();
        }
        let word = match bind.pad_action(b) {
            _ if matches!(b, B::LeftTrigger | B::RightTrigger) => "Emote",
            Some(Action::Normal) => "Topspin",
            Some(Action::Cut) => "Slice",
            Some(Action::Lob) => "Lob",
            Some(Action::Camera) if camera => "Camera",
            Some(Action::Pause) => "Pause",
            _ => "",
        };
        word.into()
    })
}

/// Whether the legend's `act` is pad button `b`'s (as `legend::glyphs` draws it).
fn on_button(act: Act, b: GamepadButton) -> bool {
    use GamepadButton as B;
    match act {
        Act::Ok | Act::Select => b == B::South,
        Act::Back => b == B::East,
        Act::Page => b == B::West,
        Act::Card => b == B::North,
        Act::Costume => matches!(b, B::LeftTrigger | B::RightTrigger),
        Act::Hand => b == B::Select,
        Act::Start | Act::Name | Act::Go => b == B::Start,
        _ => false,
    }
}

/// B662: this frame's words, from the legend as drawn (`legend::Drawn`): the group with the touch pad in it, else the
/// keyboard's (whose seat the touch pad drives), else the first.
#[allow(clippy::too_many_arguments)]
fn relabel(
    drawn: Option<Res<Drawn>>,
    named: Query<(Entity, &Name), With<Gamepad>>,
    bind: Option<Res<Bindings>>,
    g: Option<Res<super::Game>>,
    pads: Option<Res<super::Pads>>,
    np: Option<Res<super::netplay::Netplay>>,
    gr: Option<Res<crate::graphics::Graphics>>,
) {
    let touch = |d: &Option<Entity>| d.is_some_and(|e| named.get(e).is_ok_and(|(_, n)| super::emote::is_touch(n)));
    let (Some(drawn), Some(bind)) = (drawn, bind) else { return };
    let group = drawn.0.iter().find(|g| g.devs.iter().any(touch)).or_else(|| drawn.0.iter().find(|g| g.devs.contains(&None))).or(drawn.0.first());
    let play = g.is_some() && !MENU.load(Ordering::Relaxed);
    let original = gr.is_some_and(|gr| gr.original);
    let camera = g.zip(pads).is_some_and(|(g, p)| super::alt_camera::offered(&g, &p, np.is_some(), original));
    let now = (play, labels(group.map_or(&[][..], |g| &g.items), play, camera, &bind));
    let mut was = LABELS.lock().unwrap();
    if *was != now {
        #[cfg(target_arch = "wasm32")]
        crate::web::touch_labels(&now.1, play);
        *was = now;
    }
}

/// Where everything sits on a window (logical px), as `web/touch.js`'s CSS lays it out: `u` is min(7 % of the width,
/// 12.5 % of the height) in landscape (portrait: `Layout::upright`).
#[derive(Clone, Debug, PartialEq)]
struct Layout {
    u: f32,
    /// Each button's box.
    keys: [Rect; 8],
    /// The match's stick area: the left 45 %, below the top 18 %.
    zone: Rect,
    /// The menus' fixed stick: its centre.
    corner: Vec2,
    /// The chat button, left of Select.
    chat: Rect,
}

impl Layout {
    /// `safe`: the safe-area insets, left, top, right, bottom.
    fn new(size: Vec2, safe: [f32; 4]) -> Layout {
        if size.y > size.x {
            return Layout::upright(size, safe);
        }
        let [l, t, r, b] = safe;
        let u = (size.x * 0.07).min(size.y * 0.125);
        let d = u * 1.25;
        // face buttons by their right and bottom edges' distance from the safe area's
        let face = |x: f32, y: f32| Rect::from_corners(Vec2::new(size.x - r - u * x - d, size.y - b - u * y - d), Vec2::new(size.x - r - u * x, size.y - b - u * y));
        let small = |left: f32| Rect::from_corners(Vec2::new(left, t + u * 0.3), Vec2::new(left + u * 1.5, t + u * 1.0));
        Layout {
            u,
            keys: [
                face(1.4, 0.3),
                face(0.2, 1.5),
                face(2.6, 1.5),
                face(1.4, 2.7),
                small(l + u * 0.4),
                small(size.x - r - u * 0.4 - u * 1.5),
                small(size.x / 2.0 - u * 1.8),
                small(size.x / 2.0 + u * 0.3),
            ],
            zone: Rect::from_corners(Vec2::new(0.0, size.y * 0.18), Vec2::new(size.x * 0.45, size.y)),
            corner: Vec2::new(l + u * 0.4 + u * 1.3, size.y - b - u * 0.4 - u * 1.3),
            chat: Rect::from_corners(Vec2::new(size.x / 2.0 - u * 3.2, t + u * 0.3), Vec2::new(size.x / 2.0 - u * 2.3, t + u * 1.0)),
        }
    }

    /// B631: a portrait window (a phone held upright), as `web/touch.js`'s portrait rules: `u` from the short side,
    /// min(12 % of the width, 7 % of the height), thumb-sized as on its side; everything in the screen's lower part,
    /// under the picture: the face buttons as in landscape, the shoulders and Select/Start in a row over them (4.1 u up
    /// from the safe area's bottom), the chat button centred under Select/Start, the match's stick in the lower half.
    fn upright(size: Vec2, [l, _, r, b]: [f32; 4]) -> Layout {
        let u = (size.x * 0.12).min(size.y * 0.07);
        let d = u * 1.25;
        let face = |x: f32, y: f32| Rect::from_corners(Vec2::new(size.x - r - u * x - d, size.y - b - u * y - d), Vec2::new(size.x - r - u * x, size.y - b - u * y));
        let row = size.y - b - u * 4.1;
        let small = |left: f32| Rect::from_corners(Vec2::new(left, row - u * 0.7), Vec2::new(left + u * 1.5, row));
        let chat = size.y - b - u * 3.2;
        Layout {
            u,
            keys: [
                face(1.4, 0.3),
                face(0.2, 1.5),
                face(2.6, 1.5),
                face(1.4, 2.7),
                small(l + u * 0.4),
                small(size.x - r - u * 0.4 - u * 1.5),
                small(size.x / 2.0 - u * 1.8),
                small(size.x / 2.0 + u * 0.3),
            ],
            zone: Rect::from_corners(Vec2::new(0.0, size.y * 0.5), Vec2::new(size.x * 0.45, size.y)),
            corner: Vec2::new(l + u * 0.4 + u * 1.3, size.y - b - u * 0.4 - u * 1.3),
            chat: Rect::from_corners(Vec2::new(size.x / 2.0 - u * 0.45, chat - u * 0.7), Vec2::new(size.x / 2.0 + u * 0.45, chat)),
        }
    }

    /// The stick base's radius: full tilt there.
    fn reach(&self) -> f32 {
        self.u * 1.3
    }

    /// The button under `p`: a touch just past an edge (0.2 u) still hits it; face buttons are circles.
    fn key_at(&self, p: Vec2) -> Option<usize> {
        let pad = self.u * 0.2;
        self.keys.iter().position(|k| if k.width() == k.height() { p.distance(k.center()) <= k.width() / 2.0 + pad } else { k.inflate(pad).contains(p) })
    }

    /// The control under `p`: a button, else the stick (`menu`: its corner circle; a match: the zone).
    fn control_at(&self, p: Vec2, menu: bool) -> Option<Control> {
        if let Some(k) = self.key_at(p) {
            return Some(Control::Key(k));
        }
        let stick = if menu { p.distance(self.corner) <= self.reach() } else { self.zone.contains(p) };
        stick.then_some(Control::Stick)
    }
}

/// B610: how far down the HUD's view the shoulder row (L1, R1, Select, Start, chat) reaches on a `size` window, in
/// percent of the view's height; 0 without the pad. The score panel's top half drops below it.
pub fn top_row(size: Vec2) -> f32 {
    if !on() {
        return 0.0;
    }
    row_bottom(size, super::widescreen::safe_insets(size), super::widescreen::view(size))
}

/// B633: the top edge of the face buttons on a `size` window (logical px from the top); `None` without the pad. The
/// split view's lower half keeps its players above them.
pub fn face_top(size: Vec2) -> Option<f32> {
    on().then(|| Layout::new(size, super::widescreen::safe_insets(size)).keys[..FACE].iter().fold(size.y, |t, k| t.min(k.min.y)))
}

fn row_bottom(size: Vec2, safe: [f32; 4], view: Rect) -> f32 {
    // upright the row sits low, over the face buttons (B631): nothing to clear at the top
    if size.y > size.x {
        return 0.0;
    }
    let l = Layout::new(size, safe);
    let bottom = l.keys[4..].iter().fold(l.chat.max.y, |b, k| b.max(k.max.y));
    ((bottom - view.min.y) / view.height() * 100.0).max(0.0)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Control {
    Key(usize),
    Stick,
}

/// The fingers on the pad and what it reports.
#[derive(Default, Debug)]
struct Fingers {
    /// Each touch on a control: the control it landed on.
    control: HashMap<u64, Control>,
    /// The fingers on each button.
    held: [Vec<u64>; 8],
    /// Each button down since that frame.
    down: [Option<u64>; 8],
    /// The stick's finger and its centre.
    stick: Option<(u64, Vec2)>,
    /// The stick's tilt, screen axes (+y down), within the unit circle.
    tilt: Vec2,
}

impl Fingers {
    /// A touch landed at `p` on frame `frame`: whether a control took it, and whether it pressed a button.
    fn land(&mut self, id: u64, p: Vec2, l: &Layout, menu: bool, frame: u64) -> (bool, bool) {
        if self.control.contains_key(&id) {
            return (true, false);
        }
        match l.control_at(p, menu) {
            Some(Control::Key(k)) => {
                self.control.insert(id, Control::Key(k));
                self.held[k].push(id);
                let new = self.down[k].is_none();
                self.down[k] = self.down[k].or(Some(frame));
                (true, new)
            }
            // one stick finger at a time; another lands on nothing
            Some(Control::Stick) if self.stick.is_none() => {
                self.control.insert(id, Control::Stick);
                // in the menus the stick sits still: its centre is the corner's
                self.stick = Some((id, if menu { l.corner } else { p }));
                self.tilt = Vec2::ZERO;
                (true, false)
            }
            _ => (false, false),
        }
    }

    /// The stick's finger is at `p`.
    fn slide(&mut self, id: u64, p: Vec2, l: &Layout) {
        if let Some((s, c)) = self.stick
            && s == id
        {
            self.tilt = ((p - c) / l.reach()).clamp_length_max(1.0);
        }
    }

    /// A touch lifted or was cancelled.
    fn lift(&mut self, id: u64) {
        match self.control.remove(&id) {
            Some(Control::Key(k)) => self.held[k].retain(|&f| f != id),
            Some(Control::Stick) => {
                self.stick = None;
                self.tilt = Vec2::ZERO;
            }
            None => {}
        }
    }

    /// Frame `frame`'s end of its touches: a button no finger holds goes up once a frame has seen it down.
    fn settle(&mut self, frame: u64) {
        for k in 0..8 {
            if self.held[k].is_empty() && self.down[k].is_some_and(|f| f < frame) {
                self.down[k] = None;
            }
        }
    }

    /// Every finger and button let go.
    fn reset(&mut self) {
        *self = Fingers::default();
    }
}

/// The pad: its gamepad entity, its fingers, whether it's on show and in menu form.
#[derive(Resource, Default)]
struct Pad {
    entity: Option<Entity>,
    connected: bool,
    shown: bool,
    menu: bool,
    frame: u64,
    fingers: Fingers,
    safe: [f32; 4],
}

/// A menu is up (the main menu, the pause or result screen): the stick keeps to its corner.
static MENU: AtomicBool = AtomicBool::new(true);

/// Marks the pad's entities (its gamepad, its drawing).
#[derive(Component)]
struct Ours;
#[derive(Component)]
struct Root;
/// The pad's camera: after `hud_gamma`'s composite (order 51), on the whole window.
#[derive(Component)]
struct PadCamera;
#[derive(Component)]
struct KeyNode(usize);
#[derive(Component)]
struct Glyph(usize);
#[derive(Component)]
struct ChatKey;
#[derive(Component)]
enum StickNode {
    Base,
    Knob,
}

pub fn plugin(app: &mut App) {
    if on() || cfg!(target_arch = "wasm32") {
        app.add_systems(PostUpdate, relabel.after(super::legend::draw)).add_systems(Last, menu_form);
    }
    if on() {
        input(app);
        app.add_systems(PostUpdate, draw.after(relabel).before(bevy::ui::UiSystems::Prepare));
    }
}

/// The pad's input side (all but its drawing).
fn input(app: &mut App) {
    app.init_resource::<Pad>()
        .add_systems(PreUpdate, drive.after(bevy::input::InputSystems).before(crate::mode::hush));
}

/// The menus' form while no match is on, or its pause or result screen or an online match's bot-or-lobby question is
/// up (`web/touch.js` `hstTouchMenu`), so taps reach its pills.
fn menu_form(w: &World) {
    let on = w.get_resource::<super::Game>().is_none_or(|g| g.match_stats.result.is_some())
        || super::front_end::paused(w)
        || w.get_resource::<super::netplay::OnlineNotice>().is_some_and(|n| n.ask);
    MENU.store(on, Ordering::Relaxed);
}

/// The fingers → the pad's gamepad; shown or not; taps on it kept from the menus.
#[allow(clippy::too_many_arguments)]
fn drive(
    mut commands: Commands,
    mut pad: ResMut<Pad>,
    mut touches: ResMut<Touches>,
    window: Query<&Window, With<PrimaryWindow>>,
    ours: Query<(), With<Ours>>,
    mut gamepads: Query<(Entity, &mut Gamepad)>,
    mut connect: MessageWriter<GamepadConnectionEvent>,
) {
    let pad = &mut *pad;
    pad.frame += 1;
    let Ok(win) = window.single() else { return };
    // a mode switch took it (`mode.rs` keeps only the pads connected): a new one
    let entity = match pad.entity.filter(|&e| ours.contains(e)) {
        Some(e) => e,
        None => {
            pad.connected = false;
            *pad.entity.insert(commands.spawn(Ours).id())
        }
    };
    let real = gamepads.iter().any(|(e, g)| e != entity && super::controls::readable(g));
    // B656: a seated touch pad stays beside the real one
    let shown = !real || pad.connected && hst_base::seats::touch_held();
    if shown != pad.shown {
        pad.shown = shown;
        pad.fingers.reset();
    }
    if !shown {
        if pad.connected {
            // B247: gone before this frame's slots and seats see it beside the real pad
            commands.entity(entity).remove::<Gamepad>();
            pad.connected = false;
        }
        return;
    }
    if !pad.connected {
        connect.write(GamepadConnectionEvent::new(entity, GamepadConnection::Connected { name: NAME.into(), vendor_id: Some(SONY), product_id: None }));
        pad.connected = true;
    }
    // M106: the insets `widescreen` follows for the view
    pad.safe = super::widescreen::safe_insets(Vec2::new(win.width(), win.height()));
    let menu = MENU.load(Ordering::Relaxed);
    if menu != pad.menu {
        pad.menu = menu;
        if let Some((id, _)) = pad.fingers.stick {
            pad.fingers.lift(id);
        }
    }
    let l = Layout::new(Vec2::new(win.width(), win.height()), pad.safe);
    let frame = pad.frame;
    let f = &mut pad.fingers;
    let mut buzz = false;
    let landed: Vec<_> = touches.iter_just_pressed().map(|t| (t.id(), t.position())).collect();
    for (id, p) in landed {
        if crate::chat::BUTTON.load(Ordering::Relaxed) && l.chat.inflate(l.u * 0.2).contains(p) {
            crate::chat::ASK.store(3, Ordering::Relaxed);
            touches.clear_just_pressed(id);
            buzz = true;
            continue;
        }
        let (took, pressed) = f.land(id, p, &l, menu, frame);
        if took {
            touches.clear_just_pressed(id);
        }
        buzz |= pressed;
    }
    for t in touches.iter() {
        f.slide(t.id(), t.position(), &l);
    }
    let gone: Vec<u64> = touches.iter_just_released().chain(touches.iter_just_canceled()).map(|t| t.id()).collect();
    for id in gone {
        f.lift(id);
    }
    // a finger that's no longer down without its end reaching us lets its control go too
    let lost: Vec<u64> = f.control.keys().copied().filter(|&id| touches.get_pressed(id).is_none()).collect();
    for id in lost {
        f.lift(id);
    }
    f.settle(frame);
    if buzz && BUZZ.load(Ordering::Relaxed) {
        #[cfg(target_os = "ios")]
        crate::ios::haptic();
        #[cfg(target_os = "android")]
        crate::android::haptic();
    }
    let Ok((_, mut g)) = gamepads.get_mut(entity) else { return };
    for (k, b) in KEYS.iter().enumerate() {
        let down = f.down[k].is_some();
        if down != g.pressed(*b) {
            if down {
                g.digital_mut().press(*b);
            } else {
                g.digital_mut().release(*b);
            }
            g.analog_mut().set(GamepadInput::Button(*b), if down { 1.0 } else { 0.0 });
        }
    }
    let tilt = f.tilt;
    let a = g.analog_mut();
    if a.get(GamepadInput::Axis(GamepadAxis::LeftStickX)) != Some(tilt.x) {
        a.set(GamepadInput::Axis(GamepadAxis::LeftStickX), tilt.x);
    }
    // a gamepad's +y is up
    if a.get(GamepadInput::Axis(GamepadAxis::LeftStickY)) != Some(-tilt.y) {
        a.set(GamepadInput::Axis(GamepadAxis::LeftStickY), -tilt.y);
    }
}

/// The colours: `web/touch.js`'s white outlines over a dark wash.
const LINE: Color = Color::srgba(1.0, 1.0, 1.0, 0x55 as f32 / 255.0);
const WASH: Color = Color::srgba(0.0, 0.0, 0.0, 0x44 as f32 / 255.0);
const ON: Color = Color::srgba(1.0, 1.0, 1.0, 0x66 as f32 / 255.0);
const BASE: Color = Color::srgba(1.0, 1.0, 1.0, 0x22 as f32 / 255.0);

/// The pad drawn as it is this frame: spawned on first show, laid out for the window and its safe area.
#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    pad: Res<Pad>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    root: Query<Entity, With<Root>>,
    mut vis: Query<&mut Visibility, With<Root>>,
    mut camera: Query<&mut Camera, With<PadCamera>>,
    loading: Query<(), With<crate::mode::Loading>>,
    mut keys: Query<(&KeyNode, &mut Node, &mut BackgroundColor), (Without<Glyph>, Without<StickNode>)>,
    mut glyphs: Query<(&Glyph, &mut Node, &mut ImageNode), (Without<KeyNode>, Without<StickNode>)>,
    mut stick: Query<(&StickNode, &mut Node, &mut Visibility), (Without<KeyNode>, Without<Glyph>, Without<Root>)>,
    mut chat: Query<(&mut Node, &mut Visibility), (With<ChatKey>, Without<KeyNode>, Without<Glyph>, Without<StickNode>, Without<Root>)>,
    // ponytail: every word ever shown stays baked (a few dozen menu words)
    mut words: Local<HashMap<String, Option<(Handle<Image>, f32)>>>,
) {
    let Ok(win) = window.single() else { return };
    let Ok(root) = root.single() else {
        if pad.shown {
            let bubble = images.add(bubble());
            let syms = std::array::from_fn(|k| images.add(symbol(k)));
            spawn(&mut commands, bubble, syms);
        }
        return;
    };
    let _ = root;
    let shown = pad.shown && loading.is_empty();
    for mut v in &mut vis {
        v.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
    }
    for mut c in &mut camera {
        if c.is_active != shown {
            c.is_active = shown;
        }
    }
    if !shown {
        return;
    }
    let l = Layout::new(Vec2::new(win.width(), win.height()), pad.safe);
    let px = Val::Px;
    let place = |n: &mut Node, r: Rect| {
        let (x, y, w, h) = (px(r.min.x), px(r.min.y), px(r.width()), px(r.height()));
        if (n.left, n.top, n.width, n.height) != (x, y, w, h) {
            (n.left, n.top, n.width, n.height) = (x, y, w, h);
        }
    };
    for (k, mut n, mut bg) in &mut keys {
        place(&mut n, l.keys[k.0]);
        let display = if k.0 == SQUARE && LABELS.lock().unwrap().0 { Display::None } else { Display::Flex };
        if n.display != display {
            n.display = display;
        }
        let r = if k.0 < FACE { BorderRadius::MAX } else { BorderRadius::all(px(l.u * 0.3)) };
        if n.border_radius != r {
            n.border_radius = r;
        }
        let c = if pad.fingers.down[k.0].is_some() || !pad.fingers.held[k.0].is_empty() { ON } else { WASH };
        bg.set_if_neq(BackgroundColor(c));
    }
    let labels = LABELS.lock().unwrap().1.clone();
    for (gl, mut n, mut img) in &mut glyphs {
        let word = words.entry(labels[gl.0].clone()).or_insert_with_key(|s| bake(&mut images, s));
        let (handle, aspect) = word.clone().unwrap_or_default();
        if img.image != handle {
            img.image = handle;
        }
        // the browser's font sizes (web/touch.js `fit`): .22 u under a face button's symbol, .3 u on the small ones,
        // shrunk to fit 85 % of the button's width
        let h = if gl.0 < FACE { l.u * 0.22 } else { l.u * 0.3 }.min(l.keys[gl.0].width() * 0.85 / aspect.max(0.01));
        let (w, h) = (px(h * aspect), px(h));
        if (n.width, n.height) != (w, h) {
            (n.width, n.height) = (w, h);
        }
    }
    for (mut n, mut v) in &mut chat {
        v.set_if_neq(if crate::chat::BUTTON.load(Ordering::Relaxed) { Visibility::Inherited } else { Visibility::Hidden });
        place(&mut n, l.chat);
        let r = BorderRadius::all(px(l.u * 0.3));
        if n.border_radius != r {
            n.border_radius = r;
        }
    }
    let f = &pad.fingers;
    let centre = if pad.menu { Some(l.corner) } else { f.stick.map(|(_, c)| c) };
    for (s, mut n, mut v) in &mut stick {
        let (d, at) = match s {
            StickNode::Base => (l.u * 2.6, centre),
            StickNode::Knob => (l.u * 1.2, centre.map(|c| c + f.tilt * l.reach())),
        };
        v.set_if_neq(if at.is_some() { Visibility::Inherited } else { Visibility::Hidden });
        if let Some(at) = at {
            place(&mut n, Rect::from_center_size(at, Vec2::splat(d)));
        }
    }
}

/// The pad's nodes: the stick's base and knob, then the buttons: a face button's symbol (`syms`), and each one's word
/// (`draw` sets them).
fn spawn(commands: &mut Commands, bubble: Handle<Image>, syms: [Handle<Image>; FACE]) {
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    let abs = || Node { position_type: PositionType::Absolute, border_radius: BorderRadius::MAX, ..default() };
    // over the composited picture, black bars included: its own (HDR) main texture cleared clear, blended on top (a
    // camera sharing the composite's would write its own copy of the bars over them); no sprites, UI only by its target
    let out = bevy::camera::CameraOutputMode::Write { blend_state: Some(bevy::render::render_resource::BlendState::PREMULTIPLIED_ALPHA_BLENDING), clear_color: ClearColorConfig::None };
    let camera = commands
        .spawn((
            Ours,
            PadCamera,
            Camera2d,
            Camera { order: 60, clear_color: ClearColorConfig::Custom(Color::NONE), output_mode: out, is_active: false, ..default() },
            bevy::camera::Hdr,
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            Msaa::Off,
            bevy::camera::visibility::RenderLayers::none(),
        ))
        .id();
    commands.spawn((Ours, Root, full, Visibility::Hidden, UiTargetCamera(camera))).with_children(|p| {
        p.spawn((StickNode::Base, Node { border: UiRect::all(Val::Px(2.0)), ..abs() }, BackgroundColor(BASE), BorderColor::all(LINE), Visibility::Hidden));
        p.spawn((StickNode::Knob, abs(), BackgroundColor(ON), Visibility::Hidden));
        for k in 0..KEYS.len() {
            let node = Node { border: UiRect::all(Val::Px(2.0)), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..abs() };
            // the small buttons at the browser's 70 % opacity
            let a = if k < FACE { 1.0 } else { 0.7 };
            p.spawn((KeyNode(k), node, BackgroundColor(WASH), BorderColor::all(LINE.with_alpha(LINE.alpha() * a)))).with_children(|b| {
                if let Some(sym) = syms.get(k) {
                    b.spawn((Node { height: Val::Percent(34.0), aspect_ratio: Some(1.0), margin: UiRect::bottom(Val::Percent(4.0)), ..default() }, ImageNode::new(sym.clone())));
                }
                b.spawn((Glyph(k), Node::default(), ImageNode::default().with_color(Color::WHITE.with_alpha(a))));
            });
        }
        // as the small buttons
        let node = Node { border: UiRect::all(Val::Px(2.0)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..abs() };
        let glyph = Node { height: Val::Percent(80.0), aspect_ratio: Some(1.0), ..default() };
        p.spawn((ChatKey, node, BackgroundColor(WASH), BorderColor::all(LINE.with_alpha(LINE.alpha() * 0.7)), Visibility::Hidden)).with_child((glyph, ImageNode::new(bubble).with_color(Color::WHITE.with_alpha(0.7))));
    });
}

/// The chat button's speech bubble, white on clear.
fn bubble() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use tiny_skia::{PathBuilder, Pixmap, Transform};
    const S: f32 = 96.0;
    let mut p = Pixmap::new(S as u32, S as u32).expect("size");
    let mut pb = PathBuilder::new();
    pb.push_oval(tiny_skia::Rect::from_xywh(S * 0.1, S * 0.14, S * 0.8, S * 0.58).expect("rect"));
    pb.move_to(S * 0.3, S * 0.68);
    pb.line_to(S * 0.2, S * 0.88);
    pb.line_to(S * 0.46, S * 0.72);
    super::menu_art::stroke_rgb(&mut p, &pb.finish().expect("path"), [255; 3], S * 0.08, Transform::identity());
    Image::new(Extent3d { width: S as u32, height: S as u32, depth_or_array_layers: 1 }, TextureDimension::D2, super::menu_art::unpremultiply(p), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
}

/// A face button's symbol, ✕ ○ □ △ by `KEYS` index, white on clear.
fn symbol(k: usize) -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use tiny_skia::{PathBuilder, Pixmap, Transform};
    const S: f32 = 96.0;
    let mut p = Pixmap::new(S as u32, S as u32).expect("size");
    let (c, r, w) = (S / 2.0, S * 0.36, S * 0.09);
    let mut pb = PathBuilder::new();
    match k {
        0 => {
            pb.move_to(c - r, c - r);
            pb.line_to(c + r, c + r);
            pb.move_to(c + r, c - r);
            pb.line_to(c - r, c + r);
        }
        1 => pb.push_circle(c, c, r),
        2 => pb.push_rect(tiny_skia::Rect::from_xywh(c - r * 0.9, c - r * 0.9, r * 1.8, r * 1.8).expect("rect")),
        _ => {
            pb.move_to(c, c - r);
            pb.line_to(c + r * 1.05, c + r * 0.75);
            pb.line_to(c - r * 1.05, c + r * 0.75);
            pb.close();
        }
    }
    super::menu_art::stroke_rgb(&mut p, &pb.finish().expect("path"), [255; 3], w, Transform::identity());
    Image::new(Extent3d { width: S as u32, height: S as u32, depth_or_array_layers: 1 }, TextureDimension::D2, super::menu_art::unpremultiply(p), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
}

/// B662: a button's word, white on clear in the menus' bold font, and its width / height (one line, ascent to
/// descent); None for a blank button.
fn bake(images: &mut Assets<Image>, s: &str) -> Option<(Handle<Image>, f32)> {
    use ab_glyph::Font;
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use tiny_skia::{Pixmap, Transform};
    const S: f32 = 96.0;
    static FONT: OnceLock<ab_glyph::FontRef<'static>> = OnceLock::new();
    let font = FONT.get_or_init(|| ab_glyph::FontRef::try_from_slice(super::menu_art::BOLD).expect("font"));
    let path = super::menu_art::text_path(font, s)?;
    let k = S / font.height_unscaled();
    let w = (path.bounds().right() * k).ceil().max(1.0) as u32 + 4;
    let mut p = Pixmap::new(w, S as u32).expect("size");
    super::menu_art::fill_rgb(&mut p, &path, [255; 3], Transform::from_scale(k, k).post_translate(2.0, font.ascent_unscaled() * k));
    let img = Image::new(Extent3d { width: w, height: S as u32, depth_or_array_layers: 1 }, TextureDimension::D2, super::menu_art::unpremultiply(p), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    Some((images.add(img), w as f32 / S))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::touch::{TouchInput, TouchPhase};

    const SIZE: Vec2 = Vec2::new(852.0, 393.0);

    /// B662: the buttons say what they do: a menu's legend words on their buttons, the match's shots by the bindings
    /// (□ blank: it does nothing in a match), Select only where it switches a camera; nothing on the loading screen.
    #[test]
    fn buttons_say_what_they_do() {
        use super::super::legend::item;
        let bind = Bindings::default();
        let menu = [item(Act::Move, "Move", &[]), item(Act::Ok, "Choose", &[]), item(Act::Back, "Back", &[]), item(Act::Costume, "Page", &[])];
        assert_eq!(labels(&menu, false, false, &bind), ["Choose", "Back", "", "", "Page", "Page", "", ""]);
        assert_eq!(labels(&[], true, false, &bind), ["Topspin", "Slice", "", "Lob", "Emote", "Emote", "", "Pause"]);
        assert_eq!(labels(&[], true, true, &bind)[6], "Camera");
        // rebound: the words follow
        let (b, _) = Bindings::parse("topspin = PadEast\nslice = PadSouth\n");
        assert_eq!(labels(&[], true, false, &b)[..2], ["Slice", "Topspin"]);
        assert_eq!(labels(&[], false, false, &bind), [""; 8]);
    }

    fn layout() -> Layout {
        // an iPhone 15 Pro on its side: the Dynamic Island's side and the home indicator kept clear
        Layout::new(SIZE, [59.0, 0.0, 59.0, 21.0])
    }

    /// B610: with the pad on, the score panel's top Sets/Games strips (either corner, background and text) clear the
    /// shoulder row and the chat button, landscape and portrait, with and without safe-area insets.
    #[test]
    fn score_strip_clears_the_shoulders() {
        use super::super::{panel, widescreen};
        for (size, safe) in [
            (Vec2::new(1280.0, 720.0), [0.0; 4]),
            (SIZE, [59.0, 0.0, 59.0, 21.0]),
            (Vec2::new(1024.0, 768.0), [0.0; 4]),
            (Vec2::new(450.0, 900.0), [0.0; 4]),
            (Vec2::new(393.0, 852.0), [0.0, 59.0, 0.0, 34.0]),
        ] {
            let [l, t, r, b] = safe;
            let view = Rect::from_corners(Vec2::new(l, t), size - Vec2::new(r, b));
            let aspect = view.width() / view.height();
            let drop = panel::drop_below(row_bottom(size, safe, view), aspect);
            let lay = Layout::new(size, safe);
            let px = |[x, y, w, h]: [f32; 4]| {
                let (left, width) = widescreen::pin(x, w, widescreen::share(aspect));
                let (top, height) = panel::vplace_clear(y, h, aspect, drop);
                Rect::from_corners(view.min + Vec2::new(left, top) / 100.0 * view.size(), view.min + Vec2::new(left + width, top + height) / 100.0 * view.size())
            };
            for strip in [[0.0, 16.0, 240.0, 20.0], [16.0, 14.0, 184.0, 24.0], [400.0, 16.0, 240.0, 20.0], [440.0, 14.0, 184.0, 24.0]] {
                let s = px(strip);
                for k in lay.keys[4..].iter().chain([&lay.chat]) {
                    assert!(s.intersect(*k).is_empty(), "{size} {strip:?} {k:?} {s:?}");
                }
                // and not pushed far into the court
                assert!(s.min.y < lay.zone.min.y, "{size} {strip:?}");
            }
        }
    }

    #[test]
    fn layout_follows_the_browser() {
        let l = layout();
        assert_eq!(l.u, (852.0f32 * 0.07).min(393.0 * 0.125));
        // ✕ at the bottom of the diamond, its right edge 1.4 u in from the safe area's
        assert!((l.keys[0].max.x - (SIZE.x - 59.0 - 1.4 * l.u)).abs() < 1e-3);
        assert!((l.keys[0].max.y - (SIZE.y - 21.0 - 0.3 * l.u)).abs() < 1e-3);
        // △ straight above it, ○ right, □ left
        assert!((l.keys[3].center().x - l.keys[0].center().x).abs() < 1e-3 && l.keys[3].center().y < l.keys[0].center().y);
        assert!(l.keys[1].center().x > l.keys[0].center().x && l.keys[2].center().x < l.keys[0].center().x);
        // no two buttons' touch areas overlap, all inside the window
        for i in 0..8 {
            assert!(l.keys[i].min.cmpge(Vec2::ZERO).all() && l.keys[i].max.cmple(SIZE).all(), "{i}");
            for j in 0..8 {
                if i != j {
                    assert_ne!(l.key_at(l.keys[i].center()), Some(j));
                }
            }
            assert_eq!(l.key_at(l.keys[i].center()), Some(i));
        }
        // the chat button's touch area clear of every button's
        assert!((0..8).all(|i| l.keys[i].inflate(l.u * 0.2).intersect(l.chat.inflate(l.u * 0.2)).is_empty()));
        // the shoulders sit above the stick's zone; the menus' stick circle inside the window
        assert!(l.keys[4].max.y < l.zone.min.y);
        assert!(l.corner.x - l.reach() >= 59.0 && l.corner.y + l.reach() <= SIZE.y - 21.0);
    }

    /// B631: upright, the pad is as big as on its side (sized from the short side) and all of it sits in the screen's
    /// lower part, under the picture, as the browser's portrait rules have it.
    #[test]
    fn upright_pad_is_thumb_sized_and_low() {
        let size = Vec2::new(393.0, 852.0);
        let safe = [0.0, 59.0, 0.0, 34.0];
        let l = Layout::new(size, safe);
        assert_eq!(l.u, (393.0f32 * 0.12).min(852.0 * 0.07));
        assert!(l.u > layout().u * 0.9, "{} vs {}", l.u, layout().u);
        // ✕ at the bottom of the diamond, as in landscape
        assert!((l.keys[0].max.x - (size.x - 1.4 * l.u)).abs() < 1e-3);
        assert!((l.keys[0].max.y - (size.y - 34.0 - 0.3 * l.u)).abs() < 1e-3);
        // shoulders and Select/Start in one row 4.1 u up, over the face buttons; chat centred under Select/Start
        for k in &l.keys[4..] {
            assert!((k.max.y - (size.y - 34.0 - 4.1 * l.u)).abs() < 1e-3 && k.max.y < l.keys[3].min.y);
        }
        assert!((l.chat.center().x - size.x / 2.0).abs() < 1e-3 && l.chat.min.y > l.keys[6].max.y);
        // every button inside the window's lower half, each its own under its centre
        for i in 0..8 {
            assert!(l.keys[i].min.cmpge(Vec2::new(0.0, size.y / 2.0)).all() && l.keys[i].max.cmple(size).all(), "{i}");
            assert_eq!(l.key_at(l.keys[i].center()), Some(i));
        }
        assert!(l.chat.min.y > size.y / 2.0 && (0..8).all(|i| l.keys[i].intersect(l.chat).is_empty()));
        assert!(l.zone.min.y == size.y / 2.0 && l.corner.y + l.reach() <= size.y - 34.0 && l.corner.x - l.reach() >= 0.0);
        // nothing at the top for the score panel to clear
        assert_eq!(row_bottom(size, safe, Rect::from_corners(Vec2::new(0.0, 59.0), size - Vec2::new(0.0, 34.0))), 0.0);
    }

    #[test]
    fn fingers_each_count_once_and_taps_last_a_frame() {
        let l = layout();
        let mut f = Fingers::default();
        let cross = l.keys[0].center();
        // a tap within one frame: down this frame, up only after the next has seen it
        assert_eq!(f.land(1, cross, &l, false, 10), (true, true));
        f.lift(1);
        f.settle(10);
        assert!(f.down[0].is_some());
        f.settle(11);
        assert!(f.down[0].is_none());
        // two fingers on ✕: the second neither presses again nor, lifting, lets go
        f.land(2, cross, &l, false, 20);
        assert_eq!(f.land(3, cross + Vec2::X, &l, false, 21), (true, false));
        f.lift(3);
        f.settle(22);
        assert!(f.down[0].is_some());
        f.lift(2);
        f.settle(23);
        assert!(f.down[0].is_none());
        // a tap on nothing isn't taken
        assert_eq!(f.land(4, Vec2::new(SIZE.x * 0.6, SIZE.y * 0.5), &l, false, 30), (false, false));
    }

    #[test]
    fn stick_follows_the_thumb_in_a_match_and_keeps_its_corner_in_menus() {
        let l = layout();
        let mut f = Fingers::default();
        let at = Vec2::new(200.0, 250.0);
        assert!(f.land(1, at, &l, false, 1).0);
        f.slide(1, at + Vec2::new(l.reach() * 0.5, 0.0), &l);
        assert!((f.tilt - Vec2::new(0.5, 0.0)).length() < 1e-5);
        // past the base: full tilt
        f.slide(1, at + Vec2::new(0.0, -l.reach() * 3.0), &l);
        assert!((f.tilt - Vec2::new(0.0, -1.0)).length() < 1e-5);
        // a second thumb on the zone takes nothing
        assert!(!f.land(2, at + Vec2::splat(30.0), &l, false, 2).0);
        f.lift(1);
        assert_eq!(f.tilt, Vec2::ZERO);
        // menus: only the corner circle, centred there wherever the thumb lands in it
        assert!(!f.land(3, at, &l, true, 3).0);
        assert!(f.land(4, l.corner + Vec2::new(10.0, 0.0), &l, true, 3).0);
        f.slide(4, l.corner + Vec2::new(l.reach() * 0.25, 0.0), &l);
        assert!((f.tilt.x - 0.25).abs() < 1e-5);
    }

    /// The pad as the app reads it: a gamepad named for `emote::touch`, its presses kept from the menus' `Touches`,
    /// gone in the frame a real pad connects.
    #[test]
    fn a_gamepad_the_menus_dont_see_and_a_real_pad_replaces() {
        let _held = HELD.lock().unwrap_or_else(|e| e.into_inner());
        hst_base::seats::hold_touch(false);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, bevy::window::WindowPlugin::default()));
        app.insert_resource(super::super::controls::Bindings::default());
        input(&mut app);
        let win = app.world_mut().query_filtered::<Entity, With<PrimaryWindow>>().single(app.world()).unwrap();
        app.world_mut().query::<&mut Window>().single_mut(app.world_mut()).unwrap().resolution.set(SIZE.x, SIZE.y);
        app.update();
        app.update();
        let touch = |app: &mut App, phase, p: Vec2, id| {
            app.world_mut().write_message(TouchInput { phase, position: p, window: win, force: None, id });
        };
        // (entity, name, ✕ just pressed, ✕ held, the left stick)
        let pad = |app: &mut App| {
            let mut q = app.world_mut().query::<(Entity, &Gamepad, &Name)>();
            q.iter(app.world()).map(|(e, g, n)| (e, n.to_string(), g.just_pressed(GamepadButton::South), g.pressed(GamepadButton::South), g.left_stick())).next()
        };
        let (e, name, ..) = pad(&mut app).expect("connected");
        assert!(name.starts_with("Touch controls"));
        let l = Layout::new(SIZE, [0.0; 4]);
        // ✕ tapped within one frame
        touch(&mut app, TouchPhase::Started, l.keys[0].center(), 1);
        touch(&mut app, TouchPhase::Ended, l.keys[0].center(), 1);
        app.update();
        let (_, _, just, held, _) = pad(&mut app).unwrap();
        assert!(just && held);
        assert!(!app.world().resource::<Touches>().any_just_pressed());
        app.update();
        assert!(!pad(&mut app).unwrap().3);
        // a tap away from the controls reaches the menus
        touch(&mut app, TouchPhase::Started, Vec2::new(SIZE.x * 0.6, 100.0), 2);
        app.update();
        assert!(app.world().resource::<Touches>().just_pressed(2));
        touch(&mut app, TouchPhase::Ended, Vec2::new(SIZE.x * 0.6, 100.0), 2);
        // the menus' stick, pushed right
        touch(&mut app, TouchPhase::Started, l.corner, 3);
        app.update();
        touch(&mut app, TouchPhase::Moved, l.corner + Vec2::new(l.reach(), 0.0), 3);
        app.update();
        assert!((pad(&mut app).unwrap().4 - Vec2::X).length() < 1e-5);
        // a controller connects: the touch pad's gone in the frame its Gamepad shows up
        let real = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(GamepadConnectionEvent::new(real, GamepadConnection::Connected { name: "Xbox".into(), vendor_id: Some(0x045e), product_id: Some(1) }));
        app.update();
        let names: Vec<String> = app.world_mut().query::<(&Gamepad, &Name)>().iter(app.world()).map(|(_, n)| n.to_string()).collect();
        assert_eq!(names, ["Xbox"]);
        assert!(app.world().get::<Gamepad>(e).is_none());
    }

    /// `seats::hold_touch` is global: the tests that set it take turns.
    static HELD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// B656: the touch pad holding a seat (1P's, from the splash) stays when a controller connects, and both are read.
    #[test]
    fn a_seated_touch_pad_stays_beside_a_real_pad() {
        let _held = HELD.lock().unwrap_or_else(|e| e.into_inner());
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, bevy::window::WindowPlugin::default()));
        app.insert_resource(super::super::controls::Bindings::default());
        input(&mut app);
        app.world_mut().query::<&mut Window>().single_mut(app.world_mut()).unwrap().resolution.set(SIZE.x, SIZE.y);
        app.update();
        app.update();
        hst_base::seats::hold_touch(true);
        let real = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(GamepadConnectionEvent::new(real, GamepadConnection::Connected { name: "Xbox".into(), vendor_id: Some(0x045e), product_id: Some(1) }));
        app.update();
        app.update();
        let names = |app: &mut App| {
            let mut n: Vec<String> = app.world_mut().query::<(&Gamepad, &Name)>().iter(app.world()).map(|(_, n)| n.to_string()).collect();
            n.sort();
            n
        };
        assert_eq!(names(&mut app), [NAME, "Xbox"]);
        assert!(app.world().resource::<Pad>().shown);
        // its seat let go (back on the splash, say): the real pad has the screen to itself, as before
        hst_base::seats::hold_touch(false);
        app.update();
        assert_eq!(names(&mut app), ["Xbox"]);
        // and taking a seat again doesn't bring it back: only a pad the menus can see takes one
        hst_base::seats::hold_touch(true);
        app.update();
        assert_eq!(names(&mut app), ["Xbox"]);
        hst_base::seats::hold_touch(false);
    }
}

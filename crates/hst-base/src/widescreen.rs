//! Widescreen: the 3D view widens with the window (`camera` keeps the game's vertical field of view), while the HUD
//! stays on the PS2's 4:3 screen (its 640×448 space), centred, so its art keeps its shape. The score panel is the
//! exception: its halves are pinned to the window's edges (`pin`) so they don't float inside the 4:3 screen, and so
//! are the menus' (`place`): walls and bars span the window, side cards keep to its edges, the rest stays centred.
//! A window taller than 4:3 letterboxes the 4:3 screen top and bottom instead (`vplace`, B232): one scale both ways.
//!
//! B117: the Aspect Ratio setting (`ASPECT`) can fix the picture to 4:3, 16:9, 16:10 or 21:9: everything (3D, HUD,
//! menus) is drawn into the window's centred rect of that ratio (`view`), black bars fill the rest (`hud_gamma`'s
//! composite camera draws only there). All the layouts here take the view's aspect, not the window's.
//!
//! B314: in the browser the window is the canvas, which starts below the screen's top safe area (an iPhone's Dynamic
//! Island, `web/index.html`), so the view here is already the shorter rect under it: HUD and menus never reach it.
//! B590: the page's other insets (sideways, the island at one side and the home indicator below) come in as `SAFE`
//! like the phone apps' (`web::safe_area`), so at Auto the picture stays clear of them too.
//!
//! M106: the native phone apps (iOS, Android) draw edge to edge; `SAFE` holds the window's safe-area insets (read by
//! `follow_safe_area`). `HST_SAFE=l,t,r,b` (logical px) fakes them anywhere, for `--shot`s.
//!
//! B608 (replacing B421's Stretch): the 3D picture always fills the window, or the fixed ratio's centred rect of it
//! (`scene_view`), notch, cutout and home-indicator strips included; only the HUD and menus are inset (`view`): to the
//! device's safe area at the Safe Area setting's Auto, a margin of the user's on top (`MARGIN`), or none. Nothing is
//! distorted: the 3D view widens to its rect and the HUD keeps its 4:3 screen in its own. `hud_gamma` draws the HUD's
//! camera into the view's part of the picture. The touch pad keeps to the whole window (`touch_pad`), its buttons
//! clear of the device's safe area.
//!
//! B623: the menus run edge to edge too: their walls, bars and backdrops (and the key legend's strip, the loading
//! cover) carry on past the safe area to the picture's edges (`Bleed`, copied by `hud_gamma` onto a camera under the
//! HUD's), while their text, cards and buttons stay inside it. The phones hide their system bars (`phone_window`,
//! Android's `HstActivity`), so the safe area is only the notch, the island, the cutout and the home indicator.

use bevy::prelude::*;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

/// The Aspect Ratio setting's choices (index in `ASPECT`): Auto fills the window (B17's widescreen).
/// Stretch (B421) is no longer offered (`aspect_choices`, B608): a saved one reads as Auto, which now does the same.
pub const ASPECTS: [&str; 6] = ["Auto", "4:3", "16:9", "16:10", "21:9", "Stretch"];
const RATIOS: [f32; 5] = [0.0, 4.0 / 3.0, 16.0 / 9.0, 16.0 / 10.0, 21.0 / 9.0];
/// B421: the `ASPECTS` index of Stretch, past the offered ones.
pub const STRETCH: u8 = 5;

/// How many of `ASPECTS` the setting offers: B608 dropped Stretch everywhere, as the picture now always fills the
/// window and the Safe Area setting (`MARGIN`) says how far in the HUD and menus keep.
pub fn aspect_choices() -> usize {
    STRETCH as usize
}
/// The aspect ratio in force (an `ASPECTS` index): the settings' (`main_menu`), 4:3 in the original mode.
pub static ASPECT: AtomicU8 = AtomicU8::new(0);

/// B608: the Safe Area setting: `AUTO_MARGIN` keeps the HUD and menus inside the device's safe area (`SAFE`),
/// `NO_MARGIN` lets them reach the window's edges, n in 1..=20 keeps them n/2 % of the window's width and height in
/// from every edge as well (a TV that crops its edges), the device's insets still kept where they are bigger.
pub static MARGIN: AtomicU8 = AtomicU8::new(AUTO_MARGIN);
pub const AUTO_MARGIN: u8 = 0;
pub const NO_MARGIN: u8 = u8::MAX;

/// The centred rect of a `size` window the HUD and menus are laid out in: the picture's (`scene_view`) less the
/// safe area (`SAFE` and `MARGIN`).
pub fn view(size: Vec2) -> Rect {
    view_at(size, ASPECT.load(Ordering::Relaxed))
}

/// B608: the rect of a `size` window the 3D picture is drawn in: all of it at Auto, else the largest centred rect of
/// the fixed ratio; the safe area doesn't shrink it (the court and sky reach under a notch or a Dynamic Island). A
/// portrait window (a phone held upright) keeps all of it, for its own layout (B112).
pub fn scene_view(size: Vec2) -> Rect {
    view_in(size, ASPECT.load(Ordering::Relaxed), [0.0; 4])
}

fn view_at(size: Vec2, aspect: u8) -> Rect {
    view_in(size, aspect, insets(SAFE.each_ref().map(|a| f32::from_bits(a.load(Ordering::Relaxed))), MARGIN.load(Ordering::Relaxed)))
}

/// The UI's insets (shares of the window, as `SAFE`) for the device's `safe` ones under the Safe Area setting `margin`.
fn insets(safe: [f32; 4], margin: u8) -> [f32; 4] {
    match margin {
        NO_MARGIN => [0.0; 4],
        m => safe.map(|s| s.max(m as f32 * 0.005)),
    }
}

/// The picture's rect of a `size` window at `aspect`, less the insets `[l, t, r, b]` (shares of the window's width
/// and height); the whole picture if they would leave nothing of it.
fn view_in(size: Vec2, aspect: u8, [l, t, r, b]: [f32; 4]) -> Rect {
    let ratio = RATIOS.get(aspect as usize).copied().unwrap_or(0.0);
    let scene = if ratio == 0.0 || size.x < size.y || size.y <= 0.0 {
        Rect::from_corners(Vec2::ZERO, size)
    } else {
        let v = if size.x > size.y * ratio { Vec2::new(size.y * ratio, size.y) } else { Vec2::new(size.x, size.x / ratio) };
        Rect::from_center_size(size / 2.0, v)
    };
    let safe = Rect::from_corners(Vec2::new(l * size.x, t * size.y), Vec2::new(size.x * (1.0 - r), size.y * (1.0 - b)));
    let v = scene.intersect(safe);
    if v.width() > 0.0 && v.height() > 0.0 { v } else { scene }
}

/// B623: a UI node that runs on past the safe area to the picture's edges (a menu's wall, a bar, a backdrop), under the
/// notch, the Dynamic Island or the home indicator, where the HUD's camera doesn't draw. Its node is in percent of the
/// view (a root of the HUD's camera, or the child of a full-view one); `hud_gamma` draws a copy of it on the whole
/// picture, under all of the UI, the copies stacked by `order`. While `on`, each of its edges on the view's edge is
/// carried out to the picture's (`bleed_out`); a node already reaching past the view (the menus' own) is copied as is.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Bleed {
    pub order: i32,
    pub on: bool,
}

/// B623: the rect `[left, top, width, height]` (percent of a `size` window's view) with each edge on the view's edge
/// moved out to the picture's (`scene_view`); as it is where they're the same.
pub fn bleed_out(rect: [f32; 4], size: Vec2) -> [f32; 4] {
    bleed_in(rect, view(size), scene_view(size))
}

fn bleed_in([mut left, mut top, mut width, mut height]: [f32; 4], v: Rect, s: Rect) -> [f32; 4] {
    // on an edge: within half a percent (the menus snap theirs to whole pixels, a little short of it)
    const ON: f32 = 0.5;
    let (vw, vh) = (v.width().max(1e-3), v.height().max(1e-3));
    let [l, t, r, b] = [(v.min.x - s.min.x) / vw, (v.min.y - s.min.y) / vh, (s.max.x - v.max.x) / vw, (s.max.y - v.max.y) / vh].map(|e| e.max(0.0) * 100.0);
    if left + width >= 100.0 - ON {
        width = 100.0 + r - left;
    }
    if left <= ON {
        width += left + l;
        left = -l;
    }
    if top + height >= 100.0 - ON {
        height = 100.0 + b - top;
    }
    if top <= ON {
        height += top + t;
        top = -t;
    }
    [left, top, width, height]
}

/// B623: a rect in percent of a `size` window's view → in percent of its picture (`scene_view`).
pub fn view_to_scene([left, top, width, height]: [f32; 4], size: Vec2) -> [f32; 4] {
    let (v, s) = (view(size), scene_view(size));
    let (sw, sh) = (s.width().max(1e-3), s.height().max(1e-3));
    let (x, y) = (v.min.x - s.min.x, v.min.y - s.min.y);
    [(x + left / 100.0 * v.width()) / sw * 100.0, (y + top / 100.0 * v.height()) / sh * 100.0, width / 100.0 * v.width() / sw * 100.0, height / 100.0 * v.height() / sh * 100.0]
}

/// M106: the window's safe-area insets (left, top, right, bottom) as shares of its width and height, f32 bits: zero
/// but in the native phone apps (or with `HST_SAFE`), whose windows reach under the notch and the home indicator.
static SAFE: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];

/// The safe-area insets in logical px of a `size` window (the touch pad's layout keeps clear of them too).
pub fn safe_insets(size: Vec2) -> [f32; 4] {
    let s = SAFE.each_ref().map(|a| f32::from_bits(a.load(Ordering::Relaxed)));
    [s[0] * size.x, s[1] * size.y, s[2] * size.x, s[3] * size.y]
}

/// B623: the game's window on a phone: on iOS the home indicator fades out while the game is played and a swipe from
/// an edge needs a second one before the system takes it (as games do), the status bar off. Android's activity does
/// the same (`HstActivity`: immersive, edge to edge). Elsewhere as it is.
pub fn phone_window(w: Window) -> Window {
    #[cfg(target_os = "ios")]
    let w = Window {
        prefers_home_indicator_hidden: true,
        prefers_status_bar_hidden: true,
        preferred_screen_edges_deferring_system_gestures: bevy::window::ScreenEdge::All,
        ..w
    };
    w
}

/// `HST_SAFE=l,t,r,b`: fixed insets in logical px, instead of the phone's.
fn fake_safe() -> Option<[f32; 4]> {
    static FAKE: std::sync::OnceLock<Option<[f32; 4]>> = std::sync::OnceLock::new();
    *FAKE.get_or_init(|| {
        let v: Vec<f32> = std::env::var("HST_SAFE").ok()?.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        <[f32; 4]>::try_from(v).ok()
    })
}

/// Keeps `SAFE` to the window's safe area: the phone's own or the web page's (B590; twice a second, it changes as the phone turns), or
/// `HST_SAFE`'s.
fn follow_safe_area(window: Query<(&Window, Option<&bevy::window::RawHandleWrapper>), With<bevy::window::PrimaryWindow>>, mut frame: Local<u32>) {
    let Ok((w, handle)) = window.single() else { return };
    *frame += 1;
    if *frame % 30 != 1 {
        return;
    }
    let _ = handle;
    #[allow(unused_mut)]
    let mut px = fake_safe();
    #[cfg(target_os = "ios")]
    if px.is_none()
        && let Some(h) = handle
    {
        px = Some(crate::ios::safe_area(h));
    }
    #[cfg(target_os = "android")]
    if px.is_none() {
        px = crate::android::safe_area().map(|p| p.map(|v| v / w.scale_factor()));
    }
    #[cfg(target_arch = "wasm32")]
    if px.is_none() {
        px = crate::web::safe_area();
    }
    let Some([l, t, r, b]) = px else { return };
    #[allow(unused_mut)]
    let (mut ww, mut wh) = (w.width().max(1.0), w.height().max(1.0));
    // the page's insets are CSS px, of the canvas's own box (`screen_point`)
    #[cfg(target_arch = "wasm32")]
    if let Some(css) = crate::web::page_size() {
        (ww, wh) = (css.x, css.y);
    }
    let shares = [l / ww, t / wh, r / ww, b / wh].map(|v| v.clamp(0.0, 0.45));
    let mut changed = false;
    for (a, v) in SAFE.iter().zip(shares) {
        changed |= a.swap(v.to_bits(), Ordering::Relaxed) != v.to_bits();
    }
    if changed {
        info!("safe area {:?} in a {}×{} window", [l, t, r, b], w.width(), w.height());
    }
}

/// The HUD's view size (logical px) in `w`.
pub fn view_size(w: &Window) -> Vec2 {
    view(Vec2::new(w.width(), w.height())).size()
}

/// The HUD's view width over height.
pub fn aspect(w: &Window) -> f32 {
    let v = view_size(w);
    v.x / v.y.max(1.0)
}

/// The 3D picture's width over height (B608: its own rect's, wider than the HUD's view by the safe area).
pub fn scene_aspect(w: &Window) -> f32 {
    let v = scene_view(Vec2::new(w.width(), w.height())).size();
    v.x / v.y.max(1.0)
}

/// A point in a `size` window (or page) → the HUD's view size and the point in it.
pub fn in_view(size: Vec2, p: Vec2) -> (Vec2, Vec2) {
    let v = view(size);
    (v.size().max(Vec2::ONE), p - v.min)
}

/// The 640×448 point under a window position, on the centred 4:3 screen the menus' rows are drawn on (P61: from
/// `main_menu/pause.rs`, as the result screen takes taps too).
pub fn screen_point(win: &Window, p: Vec2) -> Vec2 {
    #[allow(unused_mut)]
    let mut size = Vec2::new(win.width().max(1.0), win.height().max(1.0));
    // the browser measures in CSS px (`pointer.rs`)
    #[cfg(target_arch = "wasm32")]
    if let Some(css) = crate::web::page_size() {
        size = css;
    }
    let (size, p) = in_view(size, p);
    let (share, v) = (share(size.x / size.y), vshare(size.x / size.y));
    let x = (p.x / size.x * 100.0 - 50.0 * (1.0 - share)) * 6.4 / share;
    Vec2::new(x, (p.y / size.y * 100.0 - 50.0 * (1.0 - v)) * 4.48 / v)
}

/// This frame's left click or first tap (as a [`screen_point`]), and whether the right button was clicked.
pub fn pointer_press(window: &Query<&Window, With<bevy::window::PrimaryWindow>>, mouse: &ButtonInput<MouseButton>, touches: &Touches) -> (Option<Vec2>, bool) {
    let Ok(win) = window.single() else { return (None, false) };
    let at = touches.iter_just_pressed().next().map(|t| t.position()).or_else(|| mouse.just_pressed(MouseButton::Left).then(|| win.cursor_position()).flatten());
    (at.map(|p| screen_point(win, p)), mouse.just_pressed(MouseButton::Right))
}

/// A HUD root on the centred 4:3 screen.
#[derive(Component)]
pub struct Screen43;

/// A HUD root node covering the centred 4:3 screen (kept fitted to the window by `fit`).
pub fn screen_43() -> (Node, Screen43) {
    let node = Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    };
    (node, Screen43)
}

/// Settings › Display › Menu Size (`main_menu`, f32 bits): a factor on the menus' uniform scale, so their whole
/// layout shrinks or grows together on the view (the extra room goes around it); the match's HUD doesn't take it.
pub static MENU_SCALE: AtomicU32 = AtomicU32::new(0x3f80_0000);

/// The Menu Size factor in force.
pub fn menu_scale() -> f32 {
    f32::from_bits(MENU_SCALE.load(Ordering::Relaxed))
}

/// The menus' `share` and `vshare` on a view `aspect` wide at Menu Size `k`: the 640×448 screen's share of the view's
/// width and height (over 1 when it's bigger than the view).
pub fn menu_shares(aspect: f32, k: f32) -> (f32, f32) {
    (share(aspect) * k, vshare(aspect) * k)
}

/// The menus' `menu_shares` on the primary window's view at the Menu Size in force ((1, 1) without one).
pub fn window_menu_shares(window: &Query<&Window, With<bevy::window::PrimaryWindow>>) -> (f32, f32) {
    window.single().map_or((1.0, 1.0), |w| menu_shares(aspect(w), menu_scale()))
}

/// Share of the window's width the 4:3 screen covers: all of it at 4:3 or narrower (narrower windows letterbox it
/// top and bottom, `vshare`).
pub fn share(aspect: f32) -> f32 {
    (4.0 / 3.0 / aspect).min(1.0)
}

/// A 640-wide screen rect `x, w` on a full-window node, at the 4:3 screen's scale (`share`): left of the screen's
/// centre it keeps its distance from the window's left edge, right of it from the right edge. Left and width in
/// percent of the window.
pub fn pin(x: f32, w: f32, share: f32) -> (f32, f32) {
    let left = if x + w / 2.0 < 320.0 { x / 6.4 * share } else { 100.0 - (640.0 - x) / 6.4 * share };
    (left, w / 6.4 * share)
}

/// Where a 640-space rect goes on a full-window node (`place`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Anchor {
    /// On the centred 4:3 screen.
    #[default]
    Centre,
    /// Pinned to its side of the window (`pin`).
    Sides,
    /// A wall or bar: its left edge kept from the window's left; it reaches the window's right edge if it ends at
    /// 640, else keeps its right edge from the left too.
    Span,
    /// M136: centred across as `Centre`, kept to the window's top or bottom edge as `Sides` (a strip in the upper
    /// middle stays at the top of a tall window).
    Top,
}

/// A 640-wide screen rect `x, w` placed by `a`: left and width in percent of the window.
pub fn place(x: f32, w: f32, share: f32, a: Anchor) -> (f32, f32) {
    match a {
        Anchor::Span => {
            let right = if x + w >= 640.0 { 100.0 } else { (x + w) / 6.4 * share };
            (x / 6.4 * share, right - x / 6.4 * share)
        }
        Anchor::Sides => pin(x, w, share),
        Anchor::Centre | Anchor::Top => (50.0 * (1.0 - share) + x / 6.4 * share, w / 6.4 * share),
    }
}

/// Share of the window's height the 4:3 screen covers: all of it at 4:3 or wider. A taller window letterboxes it
/// (B232): the HUD keeps its shape, the 3D view fills the rest.
pub fn vshare(aspect: f32) -> f32 {
    (aspect * 0.75).min(1.0)
}

/// A 448-high screen rect `y, h` placed by `a` on a window of `aspect`: top and height in percent of the window.
/// Same scale as across (`share`), so nothing stretches in a tall window: `Centre` rides the centred 4:3 screen,
/// `Sides`/`Span` keep to the window's top or bottom edge (by which half the rect's centre is in), and a full-height
/// rect (a wall, a dim) fills it.
pub fn vplace(y: f32, h: f32, aspect: f32, a: Anchor) -> (f32, f32) {
    vplace_in(y, h, vshare(aspect), a)
}

/// `vplace` with the 448-high screen covering `s` of the view's height (the menus' `menu_shares`).
pub fn vplace_in(y: f32, h: f32, s: f32, a: Anchor) -> (f32, f32) {
    let height = h / 4.48 * s;
    match a {
        Anchor::Centre => (50.0 * (1.0 - s) + y / 4.48 * s, height),
        _ if y <= 0.0 && y + h >= 448.0 => (0.0, 100.0),
        _ if y + h / 2.0 < 224.0 => (y / 4.48 * s, height),
        _ => (100.0 - (448.0 - y) / 4.48 * s, height),
    }
}

/// The primary window's view's width over height (4:3 without one).
pub fn window_aspect(window: &Query<&Window, With<bevy::window::PrimaryWindow>>) -> f32 {
    window.single().map_or(4.0 / 3.0, aspect)
}

/// Pixels (logical) per 448-space unit: the 4:3 screen's height over 448.
pub fn unit_px(w: &Window) -> f32 {
    let v = view_size(w);
    v.y.min(v.x * 0.75) / 448.0
}

/// `share` of the primary window's view (1 without one).
pub fn window_share(window: &Query<&Window, With<bevy::window::PrimaryWindow>>) -> f32 {
    window.single().map_or(1.0, |w| share(aspect(w)))
}

fn fit(window: Query<&Window>, mut q: Query<&mut Node, With<Screen43>>) {
    let Ok(w) = window.single() else { return };
    let a = aspect(w);
    let (s, v) = (share(a), vshare(a));
    for mut n in &mut q {
        let (left, width) = (Val::Percent(50.0 * (1.0 - s)), Val::Percent(100.0 * s));
        let (top, height) = (Val::Percent(50.0 * (1.0 - v)), Val::Percent(100.0 * v));
        if n.left != left || n.width != width || n.top != top || n.height != height {
            (n.left, n.width, n.top, n.height) = (left, width, top, height);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, fit.before(bevy::ui::UiSystems::Layout));
    if cfg!(any(target_os = "ios", target_os = "android", target_arch = "wasm32")) || fake_safe().is_some() {
        app.add_systems(First, follow_safe_area);
    }
}

#[test]
fn share_of_width() {
    assert_eq!(share(4.0 / 3.0), 1.0);
    assert_eq!(share(1.0), 1.0);
    assert!((share(16.0 / 9.0) - 0.75).abs() < 1e-6);
}

#[test]
fn pinned_to_edges() {
    // 4:3: the plain 640 screen
    assert_eq!(pin(16.0, 64.0, 1.0), (2.5, 10.0));
    assert_eq!(pin(560.0, 80.0, 1.0), (87.5, 12.5));
    // 16:9: left rects from the left edge, right rects end at the same distance from the right edge
    assert_eq!(pin(0.0, 64.0, 0.75), (0.0, 7.5));
    assert_eq!(pin(16.0, 64.0, 0.75), (1.875, 7.5));
    let (l, w) = pin(560.0, 80.0, 0.75);
    assert!((l + w - 100.0).abs() < 1e-4 && (w - 9.375).abs() < 1e-4);
}

#[test]
fn fixed_ratio_views() {
    let v = |w: f32, h: f32, a| view_at(Vec2::new(w, h), a);
    // Auto, and a portrait window: all of it
    assert_eq!(v(2400.0, 1080.0, 0), Rect::new(0.0, 0.0, 2400.0, 1080.0));
    assert_eq!(v(1080.0, 2400.0, 2), Rect::new(0.0, 0.0, 1080.0, 2400.0));
    // a 20:9 phone at 16:9: bars at the sides
    assert_eq!(v(2400.0, 1080.0, 2), Rect::new(240.0, 0.0, 2160.0, 1080.0));
    // 21:9 on a 16:9 window: bars top and bottom
    let r = v(1920.0, 1080.0, 4);
    assert!((r.width() - 1920.0).abs() < 1e-3 && (r.height() - 1920.0 * 9.0 / 21.0).abs() < 1e-3 && (r.center().y - 540.0).abs() < 1e-3);
    // 4:3 and 16:10 on 16:9
    assert_eq!(v(1920.0, 1080.0, 1), Rect::new(240.0, 0.0, 1680.0, 1080.0));
    assert_eq!(v(1920.0, 1080.0, 3), Rect::new(96.0, 0.0, 1824.0, 1080.0));
}

#[test]
fn views_in_the_safe_area() {
    // M106: an iPhone 15 Pro on its side, Dynamic Island left, home indicator below (852×393 pt, 59/0/59/21)
    let (size, safe) = (Vec2::new(852.0, 393.0), [59.0 / 852.0, 0.0, 59.0 / 852.0, 21.0 / 393.0]);
    let close = |a: Rect, b: Rect| (a.min - b.min).abs().max_element() < 1e-3 && (a.max - b.max).abs().max_element() < 1e-3;
    // B608: Auto: the HUD's view is the whole safe rect, the picture the whole window
    assert!(close(view_in(size, 0, insets(safe, AUTO_MARGIN)), Rect::new(59.0, 0.0, 793.0, 372.0)));
    assert_eq!(view_in(size, 0, [0.0; 4]), Rect::new(0.0, 0.0, 852.0, 393.0));
    // fixed ratios: the picture is the ratio's rect centred in the window, the HUD's view the part of it in the safe rect
    for a in [1, 2, 3, 4] {
        let p = view_in(size, a, [0.0; 4]);
        assert!((p.width() / p.height() - RATIOS[a as usize]).abs() < 1e-3 && (p.center() - size / 2.0).length() < 1e-3, "{a}");
        let v = view_in(size, a, safe);
        assert!(v.min.x >= 59.0f32.max(p.min.x) - 1e-3 && v.max.x <= 793.0f32.min(p.max.x) + 1e-3 && v.max.y <= 372.0 + 1e-3, "{a}");
    }
    // an Android phone's cutout on one side only: the view starts past it
    assert_eq!(view_in(Vec2::new(2400.0, 1080.0), 0, [0.04, 0.0, 0.0, 0.0]), Rect::new(96.0, 0.0, 2400.0, 1080.0));
    // None ignores the device's insets; a 5 % margin keeps 5 % in on every side, more where the device's are bigger
    assert_eq!(view_in(size, 0, insets(safe, NO_MARGIN)), Rect::new(0.0, 0.0, 852.0, 393.0));
    let v = view_in(Vec2::new(1920.0, 1080.0), 0, insets([0.0; 4], 10));
    assert!(close(v, Rect::new(96.0, 54.0, 1824.0, 1026.0)));
    assert!(close(view_in(size, 0, insets(safe, 10)), Rect::new(59.0, 19.65, 793.0, 372.0)));
}

#[test]
fn placed_rects() {
    // 16:9: a full-width wall fills the window, a bar's left piece keeps to the left, its last piece reaches the right
    assert_eq!(place(0.0, 640.0, 0.75, Anchor::Span), (0.0, 100.0));
    assert_eq!(place(0.0, 512.0, 0.75, Anchor::Span), (0.0, 60.0));
    assert_eq!(place(512.0, 128.0, 0.75, Anchor::Span), (60.0, 40.0));
    // centred content stays on the 4:3 screen
    assert_eq!(place(320.0, 64.0, 0.75, Anchor::Centre), (50.0, 7.5));
}

#[test]
fn tall_window_letterboxes() {
    // 3:4 (a phone upright): the 4:3 screen is 9/16 of the height, centred; nothing stretches
    let a = 3.0 / 4.0;
    assert!((vshare(a) - 0.5625).abs() < 1e-6);
    assert_eq!(vshare(16.0 / 9.0), 1.0);
    let (t, h) = vplace(0.0, 448.0, a, Anchor::Centre);
    assert!((t - 21.875).abs() < 1e-4 && (h - 56.25).abs() < 1e-4);
    // a wall fills, the top bar keeps to the top, the bottom bar to the bottom
    assert_eq!(vplace(0.0, 448.0, a, Anchor::Span), (0.0, 100.0));
    assert_eq!(vplace(0.0, 14.0, a, Anchor::Span).0, 0.0);
    let (t, h) = vplace(404.0, 32.0, a, Anchor::Span);
    assert!((t + h - (100.0 - 12.0 / 4.48 * 0.5625)).abs() < 1e-4);
    // wide: as before
    assert_eq!(vplace(44.8, 44.8, 16.0 / 9.0, Anchor::Sides), (10.0, 10.0));
}

#[test]
fn legend_strip_under_the_description_bar() {
    // B325: the menus' description bar (404..436) and the legend strip (436..448) stay stacked at any aspect
    for a in [0.4, 9.0 / 16.0, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
        let (bt, bh) = vplace(404.0, 32.0, a, Anchor::Span);
        let (lt, lh) = vplace(436.0, 12.0, a, Anchor::Span);
        assert!((bt + bh - lt).abs() < 1e-3 && (lt + lh - 100.0).abs() < 1e-3, "{a}");
    }
}

/// Places a HUD quad (percent of the view); only a real change marks the node, since a changed `Node` relayouts the
/// whole UI tree (B328: every frame otherwise).
pub fn put(node: &mut Mut<Node>, left: f32, top: f32, width: f32, height: f32) {
    let placed = Node { left: Val::Percent(left), top: Val::Percent(top), width: Val::Percent(width), height: Val::Percent(height), ..(**node).clone() };
    node.set_if_neq(placed);
}

#[cfg(test)]
mod point_tests {
    use super::*;

    /// The window → 640×448 mapping undoes the centred 4:3 placement at any aspect.
    #[test]
    fn window_to_screen() {
        let mut win = Window::default();
        win.resolution.set(1920.0, 1080.0);
        // 16:9: the 4:3 screen is the middle 1440 px
        assert_eq!(screen_point(&win, Vec2::new(240.0, 540.0)), Vec2::new(0.0, 224.0));
        assert_eq!(screen_point(&win, Vec2::new(1680.0, 1080.0)), Vec2::new(640.0, 448.0));
        win.resolution.set(800.0, 600.0);
        assert_eq!(screen_point(&win, Vec2::new(400.0, 300.0)), Vec2::new(320.0, 224.0));
    }
}

#[test]
fn walls_bleed_to_the_picture() {
    // B623: the sideways iPhone again: the HUD's view 59..793 × 0..372 of the 852×393 picture
    let (v, s) = (Rect::new(59.0, 0.0, 793.0, 372.0), Rect::new(0.0, 0.0, 852.0, 393.0));
    let px = |[l, t, w, h]: [f32; 4]| [v.min.x + l / 100.0 * v.width(), v.min.y + t / 100.0 * v.height(), w / 100.0 * v.width(), h / 100.0 * v.height()];
    let close = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-2);
    // a wall: every side out to the window's, a whole-pixel snap a little short of the edge included
    assert!(close(px(bleed_in([0.0, 0.0, 99.9, 99.8], v, s)), [0.0, 0.0, 852.0, 393.0]));
    // a top bar: out to the sides (no inset above); a bar across the bottom: down under the home indicator
    assert!(close(px(bleed_in([0.0, 0.0, 100.0, 10.0], v, s)), [0.0, 0.0, 852.0, 37.2]));
    assert!(close(px(bleed_in([0.0, 90.0, 100.0, 10.0], v, s)), [0.0, 334.8, 852.0, 58.2]));
    // a box inside the view stays as it is; one from the left edge only grows to the left
    assert_eq!(bleed_in([10.0, 10.0, 50.0, 50.0], v, s), [10.0, 10.0, 50.0, 50.0]);
    assert!(close(px(bleed_in([0.0, 20.0, 30.0, 10.0], v, s)), [0.0, 74.4, 279.2, 37.2]));
    // no safe area: nothing moves
    assert_eq!(bleed_in([0.0, 0.0, 100.0, 100.0], s, s), [0.0, 0.0, 100.0, 100.0]);
}

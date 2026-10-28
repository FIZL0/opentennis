//! M152g: the no-disc screen's "Join a game nearby": the hosts found on this network (`guest::Finder`), an address
//! typed in (keyboard, or the on-screen keys for a phone or a handheld's pad), then the host's disc coming in and the
//! restart into its lobby (`guest`). An overlay over each platform's no-disc page ([`plugin`] and [`button`]; the
//! desktop's page is [`no_disc`]). Sizes in `vmin`, so it lays out the same at any aspect ratio.

use super::guest::{self, Finder, Step};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

const BG: Color = Color::srgb(0.05, 0.12, 0.2);
const ORANGE: Color = Color::srgb(0.95, 0.55, 0.1);
const FAINT: Color = Color::srgb(0.75, 0.85, 1.0);
/// The on-screen keys, two rows; `⌫` rubs out.
const KEYS: [&str; 2] = ["1234567", "890.:⌫"];

/// The join overlay's state.
#[derive(Resource)]
pub struct Join {
    /// The overlay is up (a platform's own page leaves taps and keys alone then).
    pub open: bool,
    /// The game's folder (the download waits in its `cache/`).
    dir: PathBuf,
    /// Restarts the app into the host's lobby; what to tell the player if it can't (iOS: reopen it).
    restart: fn() -> String,
    finder: Option<Finder>,
    typed: String,
    sel: usize,
    /// The download under way: its address, how it goes, and its stop.
    getting: Option<(String, Arc<Mutex<Step>>, Arc<AtomicBool>)>,
    status: String,
    /// Just opened: the press that opened it isn't one of the overlay's.
    fresh: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Item {
    Host(guest::Host),
    Address,
    Key(char),
    Back,
    Cancel,
}

/// What's drawn; the overlay is made again when it changes.
#[derive(Default, PartialEq)]
struct View {
    status: String,
    /// Rows of items with their labels.
    rows: Vec<Vec<(Item, String)>>,
    sel: usize,
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct ItemButton(usize);
/// A platform page's "Join a game nearby" button ([`button`]).
#[derive(Component)]
pub struct JoinButton;

/// The overlay's systems; `dir` is the game's folder, `restart` the platform's restart (see [`Join`]).
pub fn plugin(app: &mut App, dir: PathBuf, restart: fn() -> String) {
    app.insert_resource(Join { open: false, dir, restart, finder: None, typed: String::new(), sel: 0, getting: None, status: String::new(), fresh: false })
        .add_systems(Update, (opened, input, poll, draw).chain());
    // dev runs: `HST_JOIN=<address>` goes straight to that host (the unattended guest test)
    if let Some(a) = std::env::var("HST_JOIN").ok().and_then(|a| guest::address(&a)) {
        let mut j = app.world_mut().resource_mut::<Join>();
        j.open = true;
        start(&mut j, a);
    }
}

/// A "Join a game nearby" button in a platform page's layout.
pub fn button(p: &mut ChildSpawnerCommands, font: f32) {
    p.spawn((
        JoinButton,
        Button,
        Node { padding: UiRect::axes(vmin(5), vmin(2)), border_radius: BorderRadius::all(vmin(2)), ..default() },
        BackgroundColor(Color::srgb(0.2, 0.4, 0.7)),
    ))
    .with_child((Text::new("Join a game nearby"), TextFont { font_size: FontSize::VMin(font), ..default() }));
}

/// Opens the overlay (a platform page's own key for it).
pub fn open(j: &mut Join) {
    if !j.open {
        j.open = true;
        j.fresh = true;
        j.finder = Some(Finder::start());
        j.sel = 0;
        j.status.clear();
    }
}

fn close(j: &mut Join) {
    if let Some((_, _, stop)) = j.getting.take() {
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    j.finder = None;
    j.open = false;
}

fn start(j: &mut Join, addr: String) {
    let (step, stop) = (Arc::new(Mutex::new(Step::Asking)), Arc::new(AtomicBool::new(false)));
    guest::get(addr.clone(), j.dir.clone(), step.clone(), stop.clone());
    j.getting = Some((addr, step, stop));
    j.finder = None;
    j.sel = 0;
}

fn opened(mut j: ResMut<Join>, buttons: Query<&Interaction, (Changed<Interaction>, With<JoinButton>)>) {
    if buttons.iter().any(|i| *i == Interaction::Pressed) {
        open(&mut j);
    }
}

fn view(j: &Join) -> View {
    if let Some((addr, step, _)) = &j.getting {
        let mb = |n: u64| (n + (1 << 19)) >> 20;
        let status = match &*step.lock().unwrap() {
            Step::Asking => format!("Asking {addr}..."),
            Step::HostPacking => "The host is getting its disc ready...".into(),
            Step::Disc(n, of) => format!("Getting the disc from the host: {} of {} MB", mb(*n), mb(*of)),
            Step::WaitingForLobby => "Got the disc. Waiting for the host to open a lobby (Online > Host)...".into(),
            Step::Ready | Step::Failed(_) => j.status.clone(),
        };
        return View { status, rows: vec![vec![(Item::Cancel, "Cancel".into())]], sel: 0 };
    }
    let hosts = j.finder.as_ref().map(Finder::hosts).unwrap_or_default();
    let mut rows: Vec<Vec<(Item, String)>> = hosts
        .into_iter()
        .map(|h| {
            let label = if h.name.is_empty() { h.addr.clone() } else { format!("{}  ({})", h.name, h.addr) };
            vec![(Item::Host(h), label)]
        })
        .collect();
    let looking = if rows.is_empty() { "Looking for games on this network... " } else { "" };
    let status = if j.status.is_empty() {
        format!("{looking}A host shows up here once it switches Adhoc on (Settings > Online & Mods). Or type the address it shows.")
    } else {
        j.status.clone()
    };
    rows.push(vec![(Item::Address, format!("Join  http://{}_", j.typed))]);
    rows.extend(KEYS.iter().map(|r| r.chars().map(|c| (Item::Key(c), c.to_string())).collect()));
    rows.push(vec![(Item::Back, "Back".into())]);
    let n = rows.iter().map(Vec::len).sum::<usize>();
    View { status, rows, sel: j.sel.min(n - 1) }
}

fn items(v: &View) -> Vec<Item> {
    v.rows.iter().flatten().map(|(i, _)| i.clone()).collect()
}

/// Keys, pads and pointers: arrows/d-pad move, Enter/A picks, Esc/B goes back, typed text goes into the address.
#[allow(clippy::too_many_arguments)]
fn input(
    mut j: ResMut<Join>,
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    pads: Query<&Gamepad>,
    buttons: Query<(&Interaction, &ItemButton), Changed<Interaction>>,
) {
    if !j.open || std::mem::take(&mut j.fresh) {
        typed.clear();
        return;
    }
    let v = view(&j);
    let all = items(&v);
    let pad = |b| pads.iter().any(|g| g.just_pressed(b));
    let mut pick = None;
    for (i, b) in &buttons {
        match i {
            Interaction::Pressed => pick = Some(b.0),
            Interaction::Hovered => j.sel = b.0,
            Interaction::None => {}
        }
    }
    // where the selection is: (row, column)
    let at = |sel: usize| {
        let mut n = 0;
        for (r, row) in v.rows.iter().enumerate() {
            if sel < n + row.len() {
                return (r, sel - n);
            }
            n += row.len();
        }
        (0, 0)
    };
    let index = |r: usize, c: usize| v.rows[..r].iter().map(Vec::len).sum::<usize>() + c.min(v.rows[r].len() - 1);
    let (r, c) = at(v.sel);
    if keys.just_pressed(KeyCode::ArrowUp) || pad(GamepadButton::DPadUp) {
        j.sel = index(r.saturating_sub(1), c);
    }
    if keys.just_pressed(KeyCode::ArrowDown) || pad(GamepadButton::DPadDown) {
        j.sel = index((r + 1).min(v.rows.len() - 1), c);
    }
    if keys.just_pressed(KeyCode::ArrowLeft) || pad(GamepadButton::DPadLeft) {
        j.sel = index(r, c.saturating_sub(1));
    }
    if keys.just_pressed(KeyCode::ArrowRight) || pad(GamepadButton::DPadRight) {
        j.sel = index(r, c + 1);
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) || pad(GamepadButton::South) {
        pick = Some(j.sel);
    }
    let back = keys.just_pressed(KeyCode::Escape) || pad(GamepadButton::East) || keys.just_pressed(KeyCode::BrowserBack);
    if j.getting.is_none() {
        for k in typed.read().filter(|k| k.state.is_pressed()) {
            match &k.logical_key {
                Key::Backspace => _ = j.typed.pop(),
                Key::Character(s) => s.chars().filter(|c| c.is_ascii_alphanumeric() || ".:-/".contains(*c)).for_each(|c| type_char(&mut j, c)),
                _ => {}
            }
        }
    } else {
        typed.clear();
    }
    if back {
        if j.getting.is_some() {
            pick = all.iter().position(|i| *i == Item::Cancel);
        } else {
            return close(&mut j);
        }
    }
    let Some(item) = pick.and_then(|p| all.get(p)) else { return };
    match item.clone() {
        Item::Host(h) => start(&mut j, h.addr),
        Item::Address => match guest::address(&j.typed) {
            Some(a) => start(&mut j, a),
            None => j.status = "Type the address the host shows (like 192.168.1.20 or 192.168.1.20:8080).".into(),
        },
        Item::Key('⌫') => _ = j.typed.pop(),
        Item::Key(c) => type_char(&mut j, c),
        Item::Back => close(&mut j),
        Item::Cancel => {
            if let Some((_, _, stop)) = j.getting.take() {
                stop.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            j.status.clear();
            j.finder = Some(Finder::start());
        }
    }
}

fn type_char(j: &mut Join, c: char) {
    if j.typed.len() < 60 {
        j.typed.push(c);
        j.status.clear();
    }
}

/// The download's end: the restart, or back to the list saying what went wrong.
fn poll(mut j: ResMut<Join>) {
    let Some((_, step, _)) = &j.getting else { return };
    let s = step.lock().unwrap().clone();
    match s {
        Step::Failed(e) => {
            j.getting = None;
            j.status = e;
            j.finder = Some(Finder::start());
        }
        Step::Ready if j.status.is_empty() => {
            info!("adhoc guest: the disc is in; restarting into the host's lobby");
            j.status = (j.restart)();
        }
        _ => {}
    }
}

fn draw(j: Res<Join>, mut last: Local<Option<View>>, mut commands: Commands, roots: Query<Entity, With<Root>>) {
    let v = j.open.then(|| view(&j));
    if *last == v {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    if let Some(v) = &v {
        spawn(&mut commands, v);
    }
    *last = v;
}

fn spawn(commands: &mut Commands, v: &View) {
    let font = |size: f32| TextFont { font_size: FontSize::VMin(size), ..default() };
    let root = Node {
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        row_gap: vmin(1.6),
        ..default()
    };
    commands.spawn((Root, root, BackgroundColor(BG), GlobalZIndex(100))).with_children(|p| {
        p.spawn((Text::new("Join a game nearby"), font(6.0), TextColor(Color::srgb(1.0, 0.85, 0.3))));
        p.spawn((Node { max_width: vmin(140), margin: UiRect::bottom(vmin(1.5)), ..default() }, Text::new(v.status.clone()), font(3.2), TextLayout::justify(Justify::Center), TextColor(FAINT)));
        let mut i = 0;
        for row in &v.rows {
            p.spawn(Node { column_gap: vmin(1.4), ..default() }).with_children(|r| {
                for (item, label) in row {
                    let key = matches!(item, Item::Key(_));
                    let on = i == v.sel;
                    let bg = if on { ORANGE } else { Color::srgb(0.12, 0.24, 0.38) };
                    let node = Node {
                        min_width: if key { vmin(8.5) } else { vmin(60) },
                        padding: UiRect::axes(vmin(2.5), vmin(1.2)),
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(vmin(1.5)),
                        ..default()
                    };
                    r.spawn((ItemButton(i), Button, node, BackgroundColor(bg))).with_child((Text::new(label.clone()), font(3.6), TextColor(if on { Color::BLACK } else { Color::WHITE })));
                    i += 1;
                }
            });
        }
    });
}

// ---------------------------------------------------------------- the desktop's no-disc page

#[derive(Component)]
struct ChooseButton;
#[derive(Component)]
struct Said;

/// The desktop's page when there's no disc yet: choose the ISO (the file dialog, then a start that imports it), or
/// join a game nearby. Never returns (both end in a restart).
pub fn no_disc(dir: PathBuf) -> ! {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { title: "OpenTennis".into(), ..default() }), ..default() }))
        .insert_resource(ClearColor(BG))
        .insert_resource(Picked(Arc::new(Mutex::new(None)), String::new(), 0))
        .add_systems(Startup, desktop_page)
        .add_systems(Update, (desktop_input, desktop_status).chain());
    plugin(&mut app, dir, desktop_restart);
    app.run();
    std::process::exit(0)
}

fn desktop_restart() -> String {
    let e = guest::restart(&[]);
    format!("Couldn't restart the game ({e}): start it again to join.")
}

/// The file dialog's answer (`Some(None)`: none picked), what the page says, and the selected button.
#[derive(Resource)]
struct Picked(Arc<Mutex<Option<Option<String>>>>, String, usize);

fn desktop_page(mut commands: Commands) {
    commands.spawn(Camera2d);
    let font = |size: f32| TextFont { font_size: FontSize::VMin(size), ..default() };
    let intro = "This remaster plays from your own copy of the original game:\n\
                 Hot Shots Tennis (USA), SCUS-97610, version 1.00.\n\n\
                 Choose its disc image (.iso): the game copies the part it needs (about 170 MB), and after that you can \
                 delete the ISO. No disc? Join a game nearby that has one: it plays from the host's.";
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: vmin(3),
            ..default()
        })
        .with_children(|p| {
            p.spawn((Text::new("OpenTennis"), font(7.0), TextColor(Color::srgb(1.0, 0.85, 0.3))));
            p.spawn((Node { max_width: vmin(140), ..default() }, Text::new(intro), font(3.4), TextLayout::justify(Justify::Center)));
            p.spawn((ChooseButton, Button, Node { padding: UiRect::axes(vmin(5), vmin(2)), border_radius: BorderRadius::all(vmin(2)), ..default() }, BackgroundColor(ORANGE)))
                .with_child((Text::new("Choose disc image"), font(4.5), TextColor(Color::BLACK)));
            button(p, 4.5);
            p.spawn((Said, Node { max_width: vmin(140), ..default() }, Text::new(""), font(3.0), TextLayout::justify(Justify::Center), TextColor(FAINT)));
        });
}

#[allow(clippy::too_many_arguments)]
fn desktop_input(
    mut j: ResMut<Join>,
    mut picked: ResMut<Picked>,
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    choose: Query<&Interaction, (Changed<Interaction>, With<ChooseButton>)>,
    mut colors: Query<(&mut BackgroundColor, Has<ChooseButton>), Or<(With<ChooseButton>, With<JoinButton>)>>,
) {
    if j.open {
        return;
    }
    let pad = |b| pads.iter().any(|g| g.just_pressed(b));
    if keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::Tab]) || pad(GamepadButton::DPadUp) || pad(GamepadButton::DPadDown) {
        picked.2 ^= 1;
    }
    let go = keys.just_pressed(KeyCode::Enter) || pad(GamepadButton::South);
    let asked = picked.0.lock().unwrap().take();
    if let Some(iso) = asked {
        match iso {
            Some(iso) => {
                info!("importing {iso}");
                let e = guest::restart(&["--import", &iso]);
                picked.1 = format!("Couldn't restart the game to import it ({e}). Start it with: hst --import <your ISO>");
            }
            None => picked.1 = "No disc image picked. Choose again, or start the game with: hst --import <your ISO>".into(),
        }
    }
    if choose.iter().any(|i| *i == Interaction::Pressed) || (go && picked.2 == 0) {
        picked.1 = "Waiting for the file dialog...".into();
        let out = picked.0.clone();
        std::thread::spawn(move || *out.lock().unwrap() = Some(crate::disc::pick()));
    } else if go && picked.2 == 1 {
        open(&mut j);
    }
    for (mut bg, is_choose) in &mut colors {
        let on = is_choose == (picked.2 == 0);
        bg.set_if_neq(BackgroundColor(if on { ORANGE } else { Color::srgb(0.2, 0.4, 0.7) }));
    }
}

fn desktop_status(picked: Res<Picked>, mut said: Query<&mut Text, With<Said>>) {
    for mut t in &mut said {
        if t.0 != picked.1 {
            t.0 = picked.1.clone();
        }
    }
}

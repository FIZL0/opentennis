//! The phones' start page (M99 on Android, then iOS): no disc yet, so it says which disc the game needs and opens the
//! system's file picker; the picked ISO is imported into `disc.pack` as on the desktop (`disc.rs`, M13), then the app
//! starts again into the main menu (the native window can only be made once per process). The picker and the restart
//! are the platform's (`android.rs`, `ios.rs`: `pick`, `taken`, `restart`).
//!
//! It wears the main menu's look (`hst`'s `main_menu/look.rs`) from what's in git, the disc's art isn't there yet: the
//! tiled lilac wall, the gold band with the hanging title tab, the outlined pills (the chosen one yellow), an off-white
//! panel with a green header and a dark rule, the legend strip at the foot. Sizes are the menu mockup's px at 720 lines.

#[cfg(target_os = "android")]
use crate::android as platform;
#[cfg(target_os = "ios")]
use crate::ios as platform;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// A desktop stand-in, so the page can be looked at (`cargo run -p hst-base --example start_page`): the picker opens
/// and closes at once.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod platform {
    pub const DONE: &str = "Done.";
    pub fn pick() -> bool {
        true
    }
    pub fn taken() -> Option<Option<std::fs::File>> {
        Some(None)
    }
    pub fn restart() {}
}

static HEAVY: &[u8] = include_bytes!("../../hst/assets/fonts/MPLUSRounded1c-ExtraBold.ttf");
static BOLD: &[u8] = include_bytes!("../../hst/assets/fonts/MPLUSRounded1c-Bold.ttf");
static LOGO: &[u8] = include_bytes!("../../hst/assets/logo.png");

/// The main menu's colours (sampled from it).
const WALL: Color = Color::srgb_u8(0xe0, 0xc3, 0xef);
const WALL_LINE: Color = Color::srgb_u8(0xd0, 0xae, 0xe3);
const GOLD: Color = Color::srgb_u8(0xcc, 0xa9, 0x33);
const INK: Color = Color::srgb_u8(0x2b, 0x28, 0x38);
const YELLOW: Color = Color::srgb_u8(0xfe, 0xd7, 0x4c);
const GREY: Color = Color::srgb_u8(0x70, 0x71, 0x86);
const GREY_WORD: Color = Color::srgb_u8(0xc9, 0xc8, 0xd8);
const GREEN: Color = Color::srgb_u8(0xc7, 0xe7, 0xbe);
const PANEL: Color = Color::srgb_u8(0xfd, 0xfb, 0xfe);
const VIOLET: Color = Color::srgb_u8(0x6b, 0x5f, 0x86);
const LEGEND: Color = Color::srgba(0.45, 0.36, 0.52, 0.85);

/// The mockup's px at 720 lines, in the screen's height: the page scales uniformly with the height, the width takes
/// the rest (the panel grows), so it holds at any aspect ratio.
fn u(px: f32) -> Val {
    Val::Vh(px / 7.2)
}

fn size(px: f32) -> FontSize {
    FontSize::Vh(px / 7.2)
}

#[derive(Resource)]
struct Start {
    pack: PathBuf,
    stage: Stage,
    /// The chosen pill (0 Choose disc, 1 Help).
    row: usize,
}

enum Stage {
    /// Waiting for a tap; the last try's error, if any.
    Idle(Option<String>),
    /// The system's picker is up.
    Picking,
    /// Copying the disc: per mille done, and the result once the thread is through.
    Importing(Arc<AtomicU32>, Arc<Mutex<Option<Result<(), String>>>>),
    /// Imported; restarting.
    Done,
}

#[derive(Resource)]
struct Words {
    intro: &'static str,
    help: &'static str,
}

#[derive(Component)]
struct Status;
#[derive(Component)]
struct Body;
#[derive(Component)]
struct Heading;
#[derive(Component)]
struct Pill(usize);
#[derive(Component)]
struct Bar;
#[derive(Component)]
struct Logo;

/// The start page until the import restarts the app; `intro` says where the ISO can come from on this platform,
/// `help` how else it gets there.
pub fn run(plugins: impl PluginGroup, pack: PathBuf, intro: &'static str, help: &'static str) -> ! {
    App::new()
        .add_plugins(plugins)
        .insert_resource(ClearColor(WALL))
        .insert_resource(Start { pack, stage: Stage::Idle(None), row: 0 })
        .insert_resource(Words { intro, help })
        .add_systems(Startup, page)
        .add_systems(Update, (tap, poll, show, look).chain())
        .run();
    std::process::exit(0)
}

fn page(mut commands: Commands, words: Res<Words>, mut fonts: ResMut<Assets<Font>>, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);
    let heavy = fonts.add(Font::from_bytes(HEAVY.to_vec()));
    let bold = fonts.add(Font::from_bytes(BOLD.to_vec()));
    let font = |f: &Handle<Font>, px: f32| TextFont { font: f.clone().into(), font_size: size(px), ..default() };
    // the menu's words outlined dark (a drop shadow is the closest Bevy UI has)
    let outline = TextShadow { offset: Vec2::splat(3.0), color: INK };
    let logo = images.add(Image::from_buffer(LOGO, bevy::image::ImageType::Extension("png"), default(), true, default(), default()).expect("logo.png"));

    commands
        .spawn((Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, ..default() }, BackgroundColor(WALL)))
        .with_children(|root| {
            // the wall's tiles
            root.spawn(Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), overflow: Overflow::clip(), ..default() }).with_children(|w| {
                for i in 0..120 {
                    w.spawn((Node { position_type: PositionType::Absolute, left: u(i as f32 * 18.0), width: u(1.5), height: percent(100), ..default() }, BackgroundColor(WALL_LINE)));
                }
                for i in 0..42 {
                    w.spawn((Node { position_type: PositionType::Absolute, top: u(i as f32 * 18.0), height: u(1.5), width: percent(100), ..default() }, BackgroundColor(WALL_LINE)));
                }
            });
            // the gold band and the title tab hanging from it
            root.spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: u(18.0), border: UiRect::bottom(u(3.0)), ..default() }, BackgroundColor(GOLD), BorderColor::all(INK)));
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: u(-4.0),
                    top: u(-4.0),
                    padding: UiRect::new(u(30.0), u(36.0), u(8.0), u(6.0)),
                    border: UiRect::all(u(3.0)),
                    border_radius: BorderRadius::bottom_right(u(22.0)),
                    ..default()
                },
                BackgroundColor(GOLD),
                BorderColor::all(INK),
            ))
            .with_child((Text::new("Get started"), font(&heavy, 40.0), TextColor(Color::WHITE), outline));
            root.spawn((Node { position_type: PositionType::Absolute, right: u(24.0), top: u(20.0), ..default() }, Text::new("No disc yet"), font(&heavy, 24.0), TextColor(Color::WHITE), outline));

            // the pills and the panel
            root.spawn(Node { flex_grow: 1.0, padding: UiRect::new(u(36.0), u(24.0), u(82.0), u(12.0)), column_gap: u(24.0), ..default() }).with_children(|row| {
                row.spawn(Node { width: u(290.0), flex_shrink: 0.0, flex_direction: FlexDirection::Column, row_gap: u(16.0), ..default() }).with_children(|col| {
                    for (i, name) in ["Choose disc", "Help"].into_iter().enumerate() {
                        col.spawn((
                            Button,
                            Pill(i),
                            Node { height: u(58.0), padding: UiRect::horizontal(u(26.0)), align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, border: UiRect::all(u(4.0)), border_radius: BorderRadius::MAX, ..default() },
                            BackgroundColor(GREY),
                            BorderColor::all(INK),
                            UiTransform::default(),
                        ))
                        .with_children(|p| {
                            p.spawn((Text::new(name), font(&heavy, 32.0), TextColor(GREY_WORD)));
                            p.spawn((Text::new(">"), font(&heavy, 32.0), TextColor(INK)));
                        });
                    }
                });
                row.spawn((
                    Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, border: UiRect::all(u(4.0)), border_radius: BorderRadius::all(u(20.0)), overflow: Overflow::clip(), ..default() },
                    BackgroundColor(PANEL),
                    BorderColor::all(INK),
                ))
                .with_children(|panel| {
                    // the green header: the logo, the big outlined word
                    panel
                        .spawn((Node { height: u(250.0), flex_shrink: 0.0, margin: UiRect::all(u(6.0)), padding: UiRect::all(u(14.0)), border_radius: BorderRadius::all(u(14.0)), justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::FlexEnd, ..default() }, BackgroundColor(GREEN)))
                        .with_children(|h| {
                            h.spawn((Heading, Text::new("Your disc"), font(&heavy, 64.0), TextColor(Color::WHITE), TextShadow { offset: Vec2::splat(4.0), color: INK }));
                            h.spawn((Logo, Node { height: u(210.0), align_self: AlignSelf::Center, ..default() }, ImageNode::new(logo.clone()), UiTransform::default()));
                        });
                    panel.spawn((Node { height: u(3.0), flex_shrink: 0.0, ..default() }, BackgroundColor(INK)));
                    panel.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, justify_content: JustifyContent::SpaceBetween, padding: UiRect::new(u(22.0), u(22.0), u(16.0), u(16.0)), ..default() }).with_children(|body| {
                        body.spawn(Node { flex_direction: FlexDirection::Column, row_gap: u(10.0), ..default() }).with_children(|t| {
                            t.spawn((Body, Text::new(words.intro), font(&bold, 22.0), TextColor(INK)));
                            t.spawn((Status, Text::new(""), font(&bold, 18.0), TextColor(VIOLET)));
                            // the copy's progress
                            t.spawn((Node { height: u(14.0), border: UiRect::all(u(2.0)), border_radius: BorderRadius::MAX, overflow: Overflow::clip(), display: Display::None, ..default() }, BorderColor::all(INK), BackgroundColor(Color::WHITE)))
                                .with_child((Bar, Node { width: percent(0), height: percent(100), ..default() }, BackgroundColor(YELLOW)));
                        });
                        body.spawn(Node { column_gap: u(16.0), ..default() }).with_children(|cards| {
                            for (label, value) in [("NEEDS", "Hot Shots Tennis (USA)\nSCUS-97610, v1.00"), ("COPIES", "About 170 MB, once.\nThen the ISO can go.")] {
                                cards
                                    .spawn((Node { flex_grow: 1.0, flex_basis: percent(0), flex_direction: FlexDirection::Column, padding: UiRect::axes(u(12.0), u(8.0)), border: UiRect::all(u(2.0)), border_radius: BorderRadius::all(u(10.0)), ..default() }, BorderColor::all(INK), BackgroundColor(Color::WHITE)))
                                    .with_children(|c| {
                                        c.spawn((Text::new(label), font(&heavy, 16.0), TextColor(VIOLET)));
                                        c.spawn((Text::new(value), font(&heavy, 20.0), TextColor(INK)));
                                    });
                            }
                        });
                    });
                });
            });
            // the legend strip
            root.spawn((Node { height: u(24.0), flex_shrink: 0.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, column_gap: u(6.0), ..default() }, BackgroundColor(LEGEND))).with_children(|l| {
                l.spawn((Node { padding: UiRect::horizontal(u(5.0)), border_radius: BorderRadius::all(u(3.0)), ..default() }, BackgroundColor(Color::WHITE))).with_child((Text::new("Tap"), font(&bold, 13.0), TextColor(INK)));
                l.spawn((Text::new("Choose"), font(&bold, 15.0), TextColor(Color::WHITE)));
            });
        });
}

/// The chosen pill yellow (breathing while it waits), the others grey; the logo's slow bob; the copy's bar.
fn look(
    time: Res<Time<Real>>,
    start: Res<Start>,
    words: Res<Words>,
    mut pills: Query<(&Pill, &mut BackgroundColor, &mut UiTransform, &Children), Without<Logo>>,
    mut texts: Query<&mut TextColor>,
    mut logo: Query<&mut UiTransform, With<Logo>>,
    mut body: Query<&mut Text, (With<Body>, Without<Heading>)>,
    mut heading: Query<&mut Text, (With<Heading>, Without<Body>)>,
    mut bar: Query<(&mut Node, &ChildOf), With<Bar>>,
    mut track: Query<&mut Node, Without<Bar>>,
) {
    let t = time.elapsed_secs();
    let idle = matches!(start.stage, Stage::Idle(_));
    for (pill, mut bg, mut tf, children) in &mut pills {
        let on = pill.0 == start.row;
        bg.0 = if on { YELLOW } else { GREY };
        tf.scale = Vec2::splat(if on && idle { 1.0 + 0.025 * (t * 3.0).sin() } else { 1.0 });
        if let Some(mut c) = children.first().and_then(|&c| texts.get_mut(c).ok()) {
            c.0 = if on { INK } else { GREY_WORD };
        }
        if let Some(mut c) = children.get(1).and_then(|&c| texts.get_mut(c).ok()) {
            c.0 = if on { INK } else { Color::NONE };
        }
    }
    for mut tf in &mut logo {
        tf.translation = Val2::new(Val::ZERO, u(4.0 * (t * 1.6).sin()));
        tf.rotation = Rot2::degrees(1.5 * (t * 0.8).sin());
    }
    let (text, title) = if start.row == 1 && idle { (words.help, "Help") } else { (words.intro, "Your disc") };
    for mut b in &mut body {
        if b.0 != text {
            b.0 = text.to_string();
        }
    }
    for mut h in &mut heading {
        if h.0 != title {
            h.0 = title.to_string();
        }
    }
    for (mut n, parent) in &mut bar {
        let done = match &start.stage {
            Stage::Importing(d, _) => Some(d.load(Ordering::Relaxed) as f32 / 10.0),
            Stage::Done => Some(100.0),
            _ => None,
        };
        n.width = percent(done.unwrap_or(0.0));
        if let Ok(mut track) = track.get_mut(parent.parent()) {
            track.display = if done.is_some() { Display::Flex } else { Display::None };
        }
    }
}

/// A pill tapped or clicked, or the keys: up/down choose, any other key presses the chosen one.
fn tap(mut start: ResMut<Start>, pills: Query<(&Pill, &Interaction), Changed<Interaction>>, mut keys: MessageReader<KeyboardInput>) {
    let mut press = None;
    for (pill, i) in &pills {
        if *i == Interaction::Pressed {
            press = Some(pill.0);
        }
    }
    for k in keys.read().filter(|k| k.state.is_pressed() && !k.repeat && k.key_code != KeyCode::BrowserBack) {
        match k.key_code {
            KeyCode::ArrowUp | KeyCode::KeyW => start.row = 0,
            KeyCode::ArrowDown | KeyCode::KeyS => start.row = 1,
            _ => press = Some(start.row),
        }
    }
    let Some(row) = press else { return };
    start.row = row;
    match start.stage {
        Stage::Idle(_) if row == 1 => {}
        Stage::Idle(_) if platform::pick() => start.stage = Stage::Picking,
        Stage::Idle(_) => start.stage = Stage::Idle(Some("The file picker didn't open.".into())),
        // the restart didn't happen (iOS can't): closing is the way back in
        Stage::Done => std::process::exit(0),
        _ => {}
    }
}

/// The picker's answer → the import on its own thread → the restart.
fn poll(mut start: ResMut<Start>) {
    match &start.stage {
        Stage::Picking => match platform::taken() {
            None => {}
            Some(None) => start.stage = Stage::Idle(None),
            Some(Some(file)) => {
                let (done, result) = (Arc::new(AtomicU32::new(0)), Arc::new(Mutex::new(None)));
                let (d, r, pack) = (done.clone(), result.clone(), start.pack.clone());
                std::thread::spawn(move || {
                    let out = import(file, &pack, &d);
                    *r.lock().unwrap() = Some(out);
                });
                start.stage = Stage::Importing(done, result);
            }
        },
        Stage::Importing(_, result) => {
            let Some(out) = result.lock().unwrap().take() else { return };
            start.stage = match out {
                Ok(()) => {
                    info!("disc imported into {}: restarting", start.pack.display());
                    platform::restart();
                    Stage::Done
                }
                Err(e) => Stage::Idle(Some(e)),
            };
        }
        _ => {}
    }
}

fn import(file: std::fs::File, pack: &Path, done: &AtomicU32) -> Result<(), String> {
    let _ = std::fs::create_dir_all(pack.parent().unwrap());
    let mut disc = hst_data::iso::Iso::from_source(Box::new(file)).map_err(|e| {
        info!("not a disc image: {e}");
        "That file isn't a disc image. Choose the Hot Shots Tennis (USA) ISO.".to_string()
    })?;
    let hash = disc
        .write_pack(pack, |i, n| done.store((i * 1000 / n.max(1)) as u32, Ordering::Relaxed))
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::InvalidData => format!("That's not the disc this game needs ({e}). It needs Hot Shots Tennis (USA), SCUS-97610, version 1.00."),
            _ => format!("Couldn't copy the disc: {e}"),
        })?;
    info!("disc pack {hash:016x}");
    Ok(())
}

fn show(start: Res<Start>, mut status: Query<&mut Text, With<Status>>) {
    let text = match &start.stage {
        Stage::Idle(None) if start.row == 0 => "Tap Choose disc to pick it.".to_string(),
        Stage::Idle(None) => String::new(),
        Stage::Idle(Some(e)) => format!("{e} Tap Choose disc to try again."),
        Stage::Picking => "Waiting for the file picker…".to_string(),
        Stage::Importing(done, _) => format!("Copying the disc: {}%", done.load(Ordering::Relaxed) / 10),
        Stage::Done => platform::DONE.to_string(),
    };
    for mut t in &mut status {
        if t.0 != text {
            t.0 = text.clone();
        }
    }
}

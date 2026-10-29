//! The phones' start page (M99 on Android, then iOS): no disc yet, so it says which disc the game needs and opens the
//! system's file picker; the picked ISO is imported into `disc.pack` as on the desktop (`disc.rs`, M13), then the app
//! starts again into the main menu (the native window can only be made once per process). The picker and the restart
//! are the platform's (`android.rs`, `ios.rs`: `pick`, `taken`, `restart`).

#[cfg(target_os = "android")]
use crate::android as platform;
#[cfg(target_os = "ios")]
use crate::ios as platform;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::ui::{ColorStop, LinearGradient};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Resource)]
struct Start {
    pack: PathBuf,
    stage: Stage,
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

#[derive(Component)]
struct Status;
#[derive(Component)]
struct Ball;
#[derive(Component)]
struct Shadow;
#[derive(Component)]
struct Pulse;

/// The start page until the import restarts the app; `intro` says where the ISO can come from on this platform.
pub fn run(plugins: impl PluginGroup, pack: PathBuf, intro: &'static str) -> ! {
    App::new()
        .add_plugins(plugins)
        .insert_resource(ClearColor(Color::srgb(0.03, 0.08, 0.14)))
        .insert_resource(Start { pack, stage: Stage::Idle(None) })
        .add_systems(Startup, move |commands: Commands| page(commands, intro))
        .add_systems(Update, (tap, poll, show, bounce).chain())
        .run();
    std::process::exit(0)
}

/// Sizes in `vmin` so the page lays out the same at any aspect ratio and resolution.
fn page(mut commands: Commands, intro: &str) {
    commands.spawn(Camera2d);
    let font = |size: f32| TextFont { font_size: FontSize::VMin(size), ..default() };
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: vmin(3),
                ..default()
            },
            // court blue at the top into the night at the bottom
            BackgroundGradient::from(LinearGradient {
                angle: LinearGradient::TO_BOTTOM,
                stops: vec![ColorStop::auto(Color::srgb(0.1, 0.3, 0.5)), ColorStop::auto(Color::srgb(0.03, 0.08, 0.14))],
                ..default()
            }),
        ))
        .with_children(|p| {
            // a ball bouncing over its shadow, above the title
            p.spawn(Node { width: vmin(12), height: vmin(16), justify_content: JustifyContent::Center, ..default() }).with_children(|b| {
                b.spawn((Shadow, Node { position_type: PositionType::Absolute, bottom: vmin(0), width: vmin(8), height: vmin(1.6), border_radius: BorderRadius::MAX, ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)), UiTransform::default()));
                b.spawn((Ball, Node { position_type: PositionType::Absolute, bottom: vmin(1), width: vmin(6), height: vmin(6), border_radius: BorderRadius::MAX, ..default() }, BackgroundColor(Color::srgb(0.85, 0.95, 0.2))));
            });
            p.spawn((Text::new("OpenTennis"), font(8.0), TextColor(Color::srgb(1.0, 0.85, 0.3))));
            p.spawn((Node { max_width: vmin(140), ..default() }, Text::new(intro), font(3.6), TextLayout::justify(Justify::Center)));
            p.spawn((
                Button,
                Pulse,
                Node { padding: UiRect::axes(vmin(5), vmin(2)), margin: UiRect::top(vmin(2)), border_radius: BorderRadius::all(vmin(2)), ..default() },
                BackgroundColor(Color::srgb(0.95, 0.55, 0.1)),
                UiTransform::default(),
            ))
            .with_child((Text::new("Choose disc image"), font(4.5), TextColor(Color::BLACK)));
            p.spawn((Status, Node { max_width: vmin(140), ..default() }, Text::new(""), font(3.2), TextLayout::justify(Justify::Center), TextColor(Color::srgb(0.75, 0.85, 1.0))));
        });
}

/// The ball's hop (|sin|, a bounce's arc), its shadow shrinking as it rises, and the button breathing while it waits.
fn bounce(time: Res<Time<Real>>, start: Res<Start>, mut ball: Query<&mut Node, With<Ball>>, mut shadow: Query<&mut UiTransform, (With<Shadow>, Without<Pulse>)>, mut pulse: Query<&mut UiTransform, (With<Pulse>, Without<Shadow>)>) {
    let t = time.elapsed_secs();
    // faster while importing, so the page shows it's busy
    let speed = if matches!(start.stage, Stage::Importing(..)) { 2.0 } else { 1.0 };
    let h = (t * std::f32::consts::PI * 0.9 * speed).sin().abs();
    for mut n in &mut ball {
        n.bottom = vmin(1.0 + 9.0 * h);
    }
    for mut s in &mut shadow {
        s.scale = Vec2::splat(1.0 - 0.5 * h);
    }
    let idle = matches!(start.stage, Stage::Idle(_));
    for mut s in &mut pulse {
        s.scale = Vec2::splat(if idle { 1.0 + 0.04 * (t * 3.0).sin() } else { 1.0 });
    }
}

/// A tap anywhere, a click or any button (a pad's arrive as key presses) opens the picker.
fn tap(mut start: ResMut<Start>, touches: Res<Touches>, mouse: Res<ButtonInput<MouseButton>>, mut keys: MessageReader<KeyboardInput>) {
    let key = keys.read().any(|k| k.state.is_pressed() && !k.repeat && k.key_code != KeyCode::BrowserBack);
    if !(touches.any_just_released() || mouse.any_just_released([MouseButton::Left]) || key) {
        return;
    }
    match start.stage {
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
        Stage::Idle(None) => "Tap anywhere or press a button to choose.".to_string(),
        Stage::Idle(Some(e)) => format!("{e}\n\nTap to choose again."),
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

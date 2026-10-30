//! iOS: after a screenshot (or a trip out of the app) the system keeps event focus, and GameController reports every
//! button up and the sticks centred until a touch gives it back; the app can't take it back itself. With a controller
//! in, "Tap screen to resume controller" shows over everything until the first touch, or the first pad press that
//! comes through (iOS gave focus back by itself). Draw only: no input or game state changes. The notices come from
//! `ios.rs` (`notice`); the flag's rules are here, tested anywhere.

use bevy::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A notice not yet seen by `watch`: the controllers connected then, plus one (0: none).
static NOTICE: AtomicUsize = AtomicUsize::new(0);

/// A screenshot or a resign/become-active notice, with `controllers` GCControllers connected (any thread).
pub fn notice(controllers: usize) {
    NOTICE.fetch_max(controllers + 1, Ordering::Relaxed);
}

/// The flag: the overlay is up.
#[derive(Resource, Default, Debug, PartialEq)]
pub struct NeedsTap(pub bool);

impl NeedsTap {
    /// A notice: up only with a controller to resume.
    fn notice(&mut self, controllers: usize) {
        if controllers > 0 {
            self.0 = true;
        }
    }

    /// A touch or a pad press came through: focus is back.
    fn input(&mut self, touched: bool, pressed: bool) {
        if touched || pressed {
            self.0 = false;
        }
    }
}

#[derive(Component)]
struct Overlay;

pub fn plugin(app: &mut App) {
    app.init_resource::<NeedsTap>().add_systems(PreUpdate, watch.after(bevy::input::InputSystems)).add_systems(Update, show);
}

/// Raw touches, not `Touches`: the on-screen pad takes its presses back from that.
fn watch(mut tap: ResMut<NeedsTap>, mut touches: MessageReader<bevy::input::touch::TouchInput>, pads: Query<&Gamepad>) {
    let touched = touches.read().any(|t| t.phase == bevy::input::touch::TouchPhase::Started);
    let pressed = pads.iter().any(|g| g.get_just_pressed().next().is_some());
    // input first: this frame's notice outlasts a press from before it
    tap.input(touched, pressed);
    if let Some(n) = NOTICE.swap(0, Ordering::Relaxed).checked_sub(1) {
        eprintln!("controller needs a tap: {}", n > 0);
        tap.notice(n);
    }
}

fn show(mut commands: Commands, tap: Res<NeedsTap>, mut overlay: Query<&mut Visibility, With<Overlay>>) {
    if !tap.is_changed() {
        return;
    }
    let vis = if tap.0 { Visibility::Visible } else { Visibility::Hidden };
    if let Ok(mut v) = overlay.single_mut() {
        v.set_if_neq(vis);
    } else if tap.0 {
        // the menus' dark dimmer and outlined white words (the default font is the menus', `main.rs`)
        let dim = Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() };
        let words = (Text::new("Tap screen to resume controller"), TextFont { font_size: FontSize::VMin(5.0), ..default() }, TextColor(Color::WHITE), TextShadow { offset: Vec2::splat(3.0), color: Color::srgb_u8(0x2b, 0x28, 0x38) });
        commands.spawn((Overlay, dim, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)), GlobalZIndex(i32::MAX - 2), Pickable::IGNORE, vis)).with_child((words, Pickable::IGNORE));
    }
}

#[cfg(test)]
mod tests {
    use super::NeedsTap;

    #[test]
    fn up_after_a_notice_with_a_controller_until_input() {
        let mut t = NeedsTap::default();
        t.notice(0);
        assert!(!t.0, "no controller: nothing to resume");
        t.notice(1);
        assert!(t.0, "a screenshot with a controller in");
        t.input(false, false);
        assert!(t.0, "released buttons and centred sticks aren't input");
        t.input(true, false);
        assert!(!t.0, "a touch");
        t.notice(2);
        t.input(false, true);
        assert!(!t.0, "a pad press that came through");
    }
}

//! The original's screen transition at the match's end: a still of the last frame drawn over the screen, fading out
//! (alpha counter/length, the counter starting at length + 2 and dropping one a tick) while the new scene runs under
//! it. 16 ticks from the match into the winners' ceremony, 60 from the ceremony into the result page (seen in the
//! original: `research/journal/2026-10-12-b38-gameset-dissolve/scripts/b38_transition.py`, `b38_hold.py`).
//!
//! The still is a screenshot of the window, taken at once (into the ceremony: three ticks later, the picture has settled then); the
//! caller waits for it (10 frames at most: no window, no renderer) and for its own loading
//! before it changes the scene, as `mode::leave`.

use bevy::platform::time::Instant;
use bevy::prelude::*;
use std::time::Duration;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

/// The match → ceremony and ceremony → result page lengths.
pub(super) const TO_CEREMONY: i32 = 16;
pub(super) const TO_PAGE: i32 = 60;
/// How long the picture settles before the still is taken: three ticks, in frames and in time (a fast display draws
/// several frames a tick).
const SETTLE_FRAMES: u32 = 3;
const SETTLE: Duration = Duration::from_millis(50);
/// The longest wait for the still, in frames.
const WAIT_FRAMES: u32 = 10;

#[derive(Resource, Default)]
pub(super) struct Dissolve {
    state: State,
    /// Frames waited for the capture.
    waited: u32,
    /// When the picture began to settle.
    since: Option<Instant>,
    t: i32,
    len: i32,
    acc: f32,
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Settling,
    Waiting,
    Captured(Handle<Image>),
    Fading(Handle<Image>),
}

#[derive(Component)]
struct Still;

impl Dissolve {
    /// Asks for the frame on screen; true once it is held (or can't be had) and the caller is `ready` with what
    /// comes next, when the caller changes the scene and the still starts fading over `len` ticks.
    pub(super) fn cut(&mut self, commands: &mut Commands, len: i32, ready: bool) -> bool {
        match std::mem::take(&mut self.state) {
            State::Captured(h) if ready => {
                (self.state, self.t, self.len, self.acc) = (State::Fading(h), len + 2, len, 0.0);
                true
            }
            State::Captured(h) => {
                self.state = State::Captured(h);
                false
            }
            // B760: what hid when the match ended (a bounce marker, the name tags) lingers a few ticks in the picture
            // (`hud_gamma`'s 25 % of the last frame), so the still is taken once that has gone: it is the picture the
            // screen holds until the cut
            State::Idle | State::Fading(_) if len == TO_CEREMONY => {
                (self.waited, self.since, self.state) = (0, Some(Instant::now()), State::Settling);
                false
            }
            State::Settling if self.waited < SETTLE_FRAMES || self.since.is_some_and(|s| s.elapsed() < SETTLE) => {
                self.waited += 1;
                self.state = State::Settling;
                false
            }
            // the cut to the page takes the still at once
            State::Idle | State::Fading(_) | State::Settling => {
                (self.waited, self.state) = (0, State::Waiting);
                commands.spawn(Screenshot::primary_window()).observe(captured);
                false
            }
            State::Waiting if self.waited < WAIT_FRAMES || !ready => {
                self.waited += 1;
                self.state = State::Waiting;
                false
            }
            State::Waiting => true,
        }
    }

    /// A still is fading.
    pub(super) fn fading(&self) -> bool {
        matches!(self.state, State::Fading(_))
    }

    /// The still's alpha this tick, if one is up.
    fn alpha(&self) -> Option<f32> {
        self.fading().then(|| (self.t as f32 / self.len as f32).min(1.0))
    }
}

fn captured(ev: On<ScreenshotCaptured>, mut d: ResMut<Dissolve>, mut images: ResMut<Assets<Image>>) {
    if matches!(d.state, State::Waiting) {
        // the capture comes back CPU-only: upload it to draw it
        let mut image = ev.image.clone();
        image.asset_usage = bevy::asset::RenderAssetUsages::default();
        d.state = State::Captured(images.add(image));
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Dissolve>().add_systems(Startup, setup).add_systems(Update, draw.after(super::step));
}

fn setup(mut commands: Commands) {
    let node = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    // over the match's overlays, under the widescreen bars and the loading cover
    commands.spawn((Still, node, ImageNode::default(), Visibility::Hidden, GlobalZIndex(60)));
}

/// Counts the still down at 60 Hz and draws it.
fn draw(time: Res<Time<Real>>, mut d: ResMut<Dissolve>, mut q: Query<(&mut ImageNode, &mut Visibility), With<Still>>) {
    if d.alpha().is_some() {
        d.acc += time.delta_secs() * 60.0;
        while d.acc >= 1.0 && d.t > 0 {
            d.acc -= 1.0;
            d.t -= 1;
        }
        if d.t <= 0 {
            d.state = State::Idle;
        }
    }
    let Ok((mut img, mut vis)) = q.single_mut() else { return };
    match (&d.state, d.alpha()) {
        (State::Fading(h), Some(a)) => {
            img.image = h.clone();
            img.color = Color::WHITE.with_alpha(a);
            *vis = Visibility::Visible;
        }
        _ => *vis = Visibility::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The counter starts at length + 2 (alpha clamped to 1 for the first ticks) and the still goes with it at 0.
    #[test]
    fn counts_down() {
        let mut d = Dissolve { state: State::Fading(Handle::default()), t: TO_PAGE + 2, len: TO_PAGE, ..default() };
        assert_eq!(d.alpha(), Some(1.0));
        d.t = 30;
        assert_eq!(d.alpha(), Some(0.5));
        d.state = State::Idle;
        assert_eq!(d.alpha(), None);
    }

    /// The cut waits for the still and for the caller: the picture settling, the capture, then `ready`; with no still
    /// (no renderer) it goes once the frames are up.
    #[test]
    fn cut_waits_for_still_and_caller() {
        let mut world = World::new();
        let mut d = Dissolve::default();
        let cut = |d: &mut Dissolve, world: &mut World, ready| d.cut(&mut world.commands(), TO_CEREMONY, ready);
        assert!(!cut(&mut d, &mut world, true));
        assert!(matches!(d.state, State::Settling));
        (d.waited, d.since) = (SETTLE_FRAMES, Some(Instant::now() - SETTLE));
        assert!(!cut(&mut d, &mut world, true));
        assert!(matches!(d.state, State::Waiting));
        d.state = State::Captured(Handle::default());
        assert!(!cut(&mut d, &mut world, false));
        assert!(cut(&mut d, &mut world, true));
        assert_eq!((d.fading(), d.t), (true, TO_CEREMONY + 2));
        // the cut to the page doesn't settle
        let mut p = Dissolve::default();
        assert!(!p.cut(&mut world.commands(), TO_PAGE, true));
        assert!(matches!(p.state, State::Waiting));
        // no still ever comes
        d.state = State::Waiting;
        d.waited = WAIT_FRAMES;
        assert!(!cut(&mut d, &mut world, false));
        assert!(cut(&mut d, &mut world, true));
    }
}

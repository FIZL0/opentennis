//! M95: what a player plays on — the platform (browser, Android, iOS, desktop) and the input they last used (keyboard
//! and mouse, controller, touch) — for the icons beside their name in an online lobby. Sent in the lobby's Hello and
//! again when the input changes (`net::Cmd::Device`); never in the match or the handshake check (M2e): it changes
//! nothing that's played.

use bevy::input::gamepad::GamepadEvent;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::touch::TouchInput;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    #[default]
    Desktop,
    Browser,
    Android,
    Ios,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    #[default]
    Keyboard,
    Controller,
    Touch,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub platform: Platform,
    pub input: Input,
}

/// This build's platform (a browser on a phone is the browser).
pub const PLATFORM: Platform = if cfg!(target_arch = "wasm32") {
    Platform::Browser
} else if cfg!(target_os = "android") {
    Platform::Android
} else if cfg!(target_os = "ios") {
    Platform::Ios
} else {
    Platform::Desktop
};

impl Device {
    /// This machine before any input: a phone's touch, else the keyboard.
    pub const fn here() -> Device {
        let input = if matches!(PLATFORM, Platform::Android | Platform::Ios) { Input::Touch } else { Input::Keyboard };
        Device { platform: PLATFORM, input }
    }
}

/// The input a frame's events say was used (`None` with none).
pub fn used(keys: bool, mouse: bool, pad: bool, touch: bool) -> Option<Input> {
    // a tap also comes as a mouse click on some platforms (the browser's emulated mouse): touch first
    match () {
        _ if touch => Some(Input::Touch),
        _ if pad => Some(Input::Controller),
        _ if keys || mouse => Some(Input::Keyboard),
        _ => None,
    }
}

/// The `Device` resource's value, for a network started outside a system (`Net::start`'s Hello).
pub static CURRENT: std::sync::Mutex<Device> = std::sync::Mutex::new(Device::here());

/// Keeps the `Device` resource on the input last used; a change goes to the lobby (`Net`) if there is one.
pub fn watch(
    mut dev: ResMut<Device>,
    mut keys: MessageReader<KeyboardInput>,
    mut mouse: MessageReader<MouseButtonInput>,
    mut pads: MessageReader<GamepadEvent>,
    mut touches: MessageReader<TouchInput>,
    net: Option<Res<crate::net::Net>>,
    mut sent: Local<Option<Device>>,
) {
    // a pad's button or a stick pushed past a deadzone, not a connection or a resting stick's noise
    let pad = pads.read().any(|e| match e {
        GamepadEvent::Button(b) => b.value > 0.5,
        GamepadEvent::Axis(a) => a.value.abs() > 0.5,
        GamepadEvent::Connection(_) => false,
    });
    let input = used(keys.read().count() > 0, mouse.read().count() > 0, pad, touches.read().count() > 0);
    if let Some(input) = input
        && dev.input != input
    {
        dev.input = input;
    }
    *CURRENT.lock().unwrap() = *dev;
    // a new network (it starts with `CURRENT`) or a change: the lobby hears it
    if let Some(n) = net
        && (n.is_added() || *sent != Some(*dev))
    {
        n.send(crate::net::Cmd::Device(*dev));
        *sent = Some(*dev);
    }
}

/// The `Device` resource and its watcher.
pub fn plugin(app: &mut App) {
    app.insert_resource(Device::here()).add_systems(PreUpdate, watch.after(bevy::input::InputSystems));
}

/// B736: an on-screen stick's axes, rounded before an online match uses or sends them. A thumb on glass never holds
/// still, so the raw stick came out a hair different nearly every tick and rollback's guess (the last tick's pad held)
/// was almost always wrong. Rounded to one of [`DIRECTIONS`] directions and [`LENGTHS`] lengths, with each held until
/// the thumb moves most of a step past it, the stick stays put through the jitter and only a real move sends a new
/// value. The sender's pad is the rounded one, locally too, so every machine runs the same.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TouchStick {
    /// The direction and length steps sent last (`None`: centred).
    held: Option<(i32, i32)>,
}

/// The rounded stick's directions round the circle (11.25° apart).
pub const DIRECTIONS: i32 = 32;
/// Its lengths from centre to full tilt (eighths of full travel); the shortest rounds to centre.
pub const LENGTHS: i32 = 8;
/// How far (in steps) the thumb moves from the held step before the next is taken: past half a step, so jitter on a
/// step's edge doesn't flip it back and forth.
const STICKY: f32 = 0.75;

impl TouchStick {
    /// The on-screen stick's tilt (`Gamepad::left_stick`: +x right, +y up, within the unit circle), before the pad
    /// driver's deadzone, rounded: the pad's stick bytes (`lx`, `ly`: 0x00 full left/up, 0x80 centre, 0xff full
    /// right/down), the driver's deadzone applied after the rounding (on the raw bytes its edge flickers too).
    pub fn round(&mut self, tilt: Vec2) -> (u8, u8) {
        const CENTRE: u8 = hst_sim::player::pad::CENTRE;
        let (x, y) = (tilt.x, tilt.y);
        let len = tilt.length().min(1.0) * LENGTHS as f32;
        if !len.is_finite() || tilt == Vec2::ZERO {
            self.held = None;
            return (CENTRE, CENTRE);
        }
        let dir = y.atan2(x).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * DIRECTIONS as f32;
        let (mut d, mut l) = self.held.unwrap_or((dir.round() as i32 % DIRECTIONS, len.round() as i32));
        let turn = (dir - d as f32 + DIRECTIONS as f32 / 2.0).rem_euclid(DIRECTIONS as f32) - DIRECTIONS as f32 / 2.0;
        if turn.abs() > STICKY {
            d = dir.round() as i32 % DIRECTIONS;
        }
        if (len - l as f32).abs() > STICKY {
            l = len.round() as i32;
        }
        if l == 0 {
            self.held = None;
            return (CENTRE, CENTRE);
        }
        self.held = Some((d, l));
        let a = d as f32 / DIRECTIONS as f32 * std::f32::consts::TAU;
        let r = l as f32 / LENGTHS as f32;
        let byte = |v: f32| {
            let v = v.clamp(-1.0, 1.0);
            hst_sim::player::pad_deadzone((128.0 + v * if v < 0.0 { 128.0 } else { 127.0 }).round() as u8)
        };
        (byte(r * a.cos()), byte(-r * a.sin()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_input_wins_touch_over_its_mouse() {
        assert_eq!(used(false, false, false, false), None);
        assert_eq!(used(true, false, false, false), Some(Input::Keyboard));
        assert_eq!(used(false, true, false, false), Some(Input::Keyboard));
        assert_eq!(used(false, false, true, false), Some(Input::Controller));
        assert_eq!(used(false, true, false, true), Some(Input::Touch));
        assert_eq!(Device::here().platform, Platform::Desktop);
    }

    #[test]
    fn small_on_the_wire() {
        let d = Device { platform: Platform::Android, input: Input::Touch };
        let s = serde_json::to_string(&d).unwrap();
        assert_eq!(s, r#"{"platform":"Android","input":"Touch"}"#);
        assert_eq!(serde_json::from_str::<Device>(&s).unwrap(), d);
    }

    /// A thumb held on the stick, wobbling a pixel or two every tick, then sliding round a quarter turn: the pad
    /// that goes out changes nearly every tick (each a rollback guess gone wrong, its ticks re-run), unless it's
    /// rounded, and then only when the thumb really moves.
    #[test]
    fn a_jittery_thumb_sends_few_changes() {
        // the pad's bytes as `pad_stick` and `stick_byte` make them from the tilt, unrounded
        let byte = |v: f32| {
            let b = (128.0 + v.clamp(-1.0, 1.0) * if v < 0.0 { 128.0 } else { 127.0 }).round() as u8;
            hst_sim::player::pad_deadzone(b)
        };
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut noise = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 2001) as f32 / 1000.0 - 1.0
        };
        // a 60 px reach: a pixel is 1/60 of full tilt
        let px = 1.0 / 60.0;
        let (mut raw_changes, mut rounded_changes) = (0, 0);
        let mut stick = TouchStick::default();
        let (mut last_raw, mut last_rounded) = ((0x80, 0x80), (0x80, 0x80));
        for t in 0..600 {
            // held to the right near full tilt, then turning to up over the last 200 ticks
            let a = if t < 400 { 0.0 } else { (t - 400).min(180) as f32 / 180.0 * std::f32::consts::FRAC_PI_2 };
            let tilt = Vec2::from_angle(a) * 0.95 + 2.0 * px * Vec2::new(noise(), noise());
            let raw = (byte(tilt.x), byte(-tilt.y));
            let rounded = stick.round(tilt);
            raw_changes += (raw != last_raw) as u32;
            rounded_changes += (rounded != last_rounded) as u32;
            (last_raw, last_rounded) = (raw, rounded);
        }
        assert!(raw_changes > 400, "{raw_changes}");
        // the first push and a few direction steps through the quarter turn (8 steps, some inside the deadzone)
        assert!(rounded_changes <= 10, "{rounded_changes} vs {raw_changes} raw");
        // it ended pointing up, at 7/8 of full travel or more
        let (lx, ly) = last_rounded;
        assert!(lx == 0x80 && ly <= 0x10, "{lx:#x} {ly:#x}");
    }

    #[test]
    fn rounding_keeps_centre_and_the_deadzone() {
        let r = |x: f32, y: f32| TouchStick::default().round(Vec2::new(x, y));
        assert_eq!(r(0.0, 0.0), (0x80, 0x80));
        // full right, full up, full left, full down
        assert_eq!(r(1.0, 0.0), (0xff, 0x80));
        assert_eq!(r(0.0, 1.0), (0x80, 0x00));
        assert_eq!(r(-1.0, 0.0), (0x00, 0x80));
        assert_eq!(r(0.0, -1.0), (0x80, 0xff));
        // a diagonal up and right
        let (x, y) = r(0.7, 0.7);
        assert!(x > 0xae && y < 0x51 && (x as i32 - 128 + (y as i32 - 128)).abs() <= 2, "{x:#x} {y:#x}");
        // a short push rounds to centre
        assert_eq!(r(0.05, 0.0), (0x80, 0x80));
        // lifting the thumb centres it at once
        let mut s = TouchStick::default();
        s.round(Vec2::X);
        assert_eq!(s.round(Vec2::ZERO), (0x80, 0x80));
        // every output byte is past the driver's deadzone or centred
        for i in 0..64 {
            let (x, y) = r((i as f32 * 0.37).cos() * 0.8, (i as f32 * 0.37).sin() * 0.8);
            for b in [x, y] {
                assert!(b == 0x80 || !(0x51..=0xae).contains(&b), "{i} -> {b:#x}");
            }
        }
    }
}
